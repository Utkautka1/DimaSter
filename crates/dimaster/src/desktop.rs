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

pub(crate) fn open(link: &str, cwd: Option<&str>) {
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
