//! Связь с рабочим столом: открытие ссылок и уведомления.

use dimaster_data::is_leveldb;
use dimaster_grid::is_web_address;
use dimaster_i18n::t;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

// без оболочки: адрес уходит одним аргументом
fn spawn(mut command: Command) {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match command.spawn() {
        // дожидаемся в фоне, чтобы не копить зомби-процессы
        Ok(mut child) => drop(thread::spawn(move || child.wait())),
        Err(e) => log::warn!(
            "{}",
            t!(
                "Не удалось запустить {program}: {error}",
                program = format!("{:?}", command.get_program()),
                error = e
            )
        ),
    }
}

// путь из текста экрана: `~/`, экранированные пробелы и номер строки
fn local_path(link: &str, cwd: Option<&str>) -> Option<PathBuf> {
    let link = link.strip_prefix("file://").unwrap_or(link);
    let link = link
        .trim_end_matches([',', ';', ':', ')'])
        .replace("\\ ", " ");
    // "src/main.rs:12" -> "src/main.rs"
    let name = link.split(':').next().filter(|n| !n.is_empty())?;
    let path = match (name.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ if name == "~" => PathBuf::from(std::env::var_os("HOME")?),
        _ => Path::new(cwd.unwrap_or(".")).join(name),
    };
    path.exists().then_some(path)
}

/// Что открыть: адрес, папку или папку файла; `cwd` — каталог shell.
///
/// Сами файлы не открываются: вывод программы не должен запускать код кликом.
pub(crate) fn target(link: &str, cwd: Option<&str>) -> Option<PathBuf> {
    if ["http://", "https://", "mailto:"]
        .iter()
        .any(|p| link.starts_with(p))
    {
        return Some(PathBuf::from(link));
    }
    match local_path(link, cwd) {
        Some(path) if path.is_dir() => Some(path),
        Some(path) => path.parent().map(Path::to_path_buf),
        None => is_web_address(link).then(|| PathBuf::from(format!("https://{link}"))),
    }
}

/// Файл с расширением из `click`: его по щелчку показывает `dimaster view`,
/// который только читает.
pub(crate) fn table_file(link: &str, cwd: Option<&str>, click: &[String]) -> Option<PathBuf> {
    let path = local_path(link, cwd)?;
    let has = |name: &str| click.iter().any(|c| c.eq_ignore_ascii_case(name));
    if path.is_dir() {
        return (has("leveldb") && is_leveldb(&path)).then_some(path);
    }
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let listed = path.is_file() && has(&extension);
    // `.db` бывает чем угодно, поэтому проверяется подпись SQLite
    let table = listed && (extension != "db" || is_sqlite(&path));
    table.then_some(path)
}

fn is_sqlite(path: &Path) -> bool {
    let mut header = [0; 16];
    let read = std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut header));
    read.is_ok() && &header == b"SQLite format 3\0"
}

// новое окно с просмотрщиком; `false`, если файл не для него или он выключен
fn view(link: &str, cwd: Option<&str>) -> bool {
    let settings = dimaster_config::Viewer::load().map(|v| v.settings);
    let Some(settings) = settings.ok().filter(|s| s.enabled) else {
        return false;
    };
    let path = table_file(link, cwd, &settings.click);
    let (Ok(exe), Some(path)) = (std::env::current_exe(), path) else {
        return false;
    };
    let mut command = Command::new(&exe);
    command.arg("-e").arg(&exe).arg("view").arg(path);
    spawn(command);
    true
}

pub(crate) fn open(link: &str, cwd: Option<&str>) {
    if view(link, cwd) {
        return;
    }
    match target(link, cwd) {
        Some(target) => {
            let mut command = Command::new("xdg-open");
            command.arg(target);
            spawn(command);
        }
        None => log::info!("{}", t!("Ссылка не открывается: {link}", link = link)),
    }
}

pub(crate) fn notify(title: &str, body: &str) {
    let mut command = Command::new("notify-send");
    command.args(["--app-name=DimaSter", "--", title, body]);
    spawn(command);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_tables_go_to_the_viewer() {
        let file = std::env::temp_dir().join(format!("dimaster-click-{}.CSV", std::process::id()));
        std::fs::write(&file, "a\n").expect("файл");
        let click = dimaster_config::ViewSettings::default().click;
        let found = table_file(&file.to_string_lossy(), None, &click);
        std::fs::remove_file(&file).expect("уборка");
        assert_eq!(found, Some(file));
        assert!(table_file("/etc/hostname", None, &click).is_none());
    }

    #[test]
    fn db_needs_the_sqlite_signature() {
        let file = std::env::temp_dir().join(format!("dimaster-click-{}.db", std::process::id()));
        let link = file.to_string_lossy().into_owned();
        std::fs::write(&file, "not a database, just text").expect("файл");
        let click = dimaster_config::ViewSettings::default().click;
        assert!(table_file(&link, None, &click).is_none());
        std::fs::write(&file, b"SQLite format 3\0").expect("файл");
        assert!(table_file(&link, None, &click).is_some());
        std::fs::remove_file(&file).expect("уборка");
    }

    #[test]
    fn files_open_their_folder_and_domains_the_browser() {
        let etc = Some(PathBuf::from("/etc"));
        assert_eq!(target("/etc/hostname:3", None), etc);
        assert_eq!(target("hostname", Some("/etc")), etc);
        assert_eq!(target("/etc", None), etc);
        let web = target("example.com", Some("/etc"));
        assert_eq!(web, Some(PathBuf::from("https://example.com")));
        assert!(target("/no/such/file", None).is_none());
        assert!(target("слово", Some("/etc")).is_none());
    }
}
