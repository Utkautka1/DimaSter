//! Связь с рабочим столом: открытие ссылок и уведомления.

use dimaster_grid::is_web_address;
use dimaster_i18n::t;
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

// только заведомо табличные файлы: README.md или package.json не они
const TABLES: [&str; 14] = [
    "csv", "tsv", "jsonl", "parquet", "pqt", "arrow", "feather", "xlsx", "xls", "xlsm", "xlsb",
    "ods", "sqlite", "sqlite3",
];

/// Файл, который по щелчку показывает `dimaster view`: он только читает.
pub(crate) fn table_file(link: &str, cwd: Option<&str>) -> Option<PathBuf> {
    let path = local_path(link, cwd).filter(|p| p.is_file())?;
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    TABLES.contains(&extension.as_str()).then_some(path)
}

// новое окно с просмотрщиком; `false`, если он выключен в view.lua
fn view(path: PathBuf) -> bool {
    let enabled = dimaster_config::Viewer::load().is_ok_and(|v| v.settings.enabled);
    let (Ok(exe), true) = (std::env::current_exe(), enabled) else {
        return false;
    };
    let mut command = Command::new(&exe);
    command.arg("-e").arg(&exe).arg("view").arg(path);
    spawn(command);
    true
}

pub(crate) fn open(link: &str, cwd: Option<&str>) {
    if table_file(link, cwd).is_some_and(view) {
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
        let found = table_file(&file.to_string_lossy(), None);
        std::fs::remove_file(&file).expect("уборка");
        assert_eq!(found, Some(file));
        assert!(table_file("/etc/hostname", None).is_none());
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
