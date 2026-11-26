//! Связь с рабочим столом: открытие ссылок и уведомления.

use std::os::unix::fs::PermissionsExt;
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
        Err(e) => log::warn!("Не удалось запустить {:?}: {e}", command.get_program()),
    }
}

// вывод программ не должен запускать код одним кликом
fn is_safe_to_open(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let launcher = path.extension().is_some_and(|e| e == "desktop");
    meta.is_dir() || (!launcher && meta.permissions().mode() & 0o111 == 0)
}

/// Адрес или существующий безопасный файл; `cwd` — каталог shell.
pub(crate) fn target(link: &str, cwd: Option<&str>) -> Option<PathBuf> {
    if ["http://", "https://", "mailto:"]
        .iter()
        .any(|p| link.starts_with(p))
    {
        return Some(PathBuf::from(link));
    }
    let link = link.strip_prefix("file://").unwrap_or(link);
    // "src/main.rs:12" -> "src/main.rs"
    let name = link.split(':').next()?;
    let path = match (name.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => Path::new(cwd.unwrap_or(".")).join(name),
    };
    is_safe_to_open(&path).then_some(path)
}

pub(crate) fn open(link: &str, cwd: Option<&str>) {
    match target(link, cwd) {
        Some(target) => {
            let mut command = Command::new("xdg-open");
            command.arg(target);
            spawn(command);
        }
        None => log::info!("Ссылка не открывается: {link}"),
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
    fn refuses_executables_and_missing_files() {
        assert!(target("https://example.com", None).is_some());
        assert!(target("/bin/sh", None).is_none());
        assert!(target("/no/such/file", None).is_none());
        assert_eq!(
            target("/etc/hostname:3", None),
            Some(PathBuf::from("/etc/hostname"))
        );
    }
}
