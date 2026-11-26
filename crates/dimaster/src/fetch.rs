//! `dimaster fetch`: сведения о системе, встроенный аналог fastfetch.
//!
//! Читает только файлы из `/proc`, `/etc` и `/var/lib`; без них строка
//! просто пропускается.

use std::io::{self, Write};
use std::process::Command;

use crate::icat;

const LOGO: [&str; 7] = [
    " ██████╗  ███████╗",
    " ██╔══██╗ ██╔════╝",
    " ██║  ██║ ███████╗",
    " ██║  ██║ ╚════██║",
    " ██████╔╝ ███████║",
    " ╚═════╝  ╚══════╝",
    "     DimaSter",
];
// ширина колонки логотипа в ячейках
const LOGO_WIDTH: usize = 22;

fn read(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

// значение строки вида `ключ: значение` или `ключ=значение`
fn field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.strip_prefix(key)?.trim_start();
        let value = rest.strip_prefix([':', '='])?;
        Some(value.trim().trim_matches('"').to_owned())
    })
}

fn uptime(seconds: u64) -> String {
    let (days, hours, minutes) = (seconds / 86400, seconds / 3600 % 24, seconds / 60 % 60);
    match (days, hours) {
        (0, 0) => format!("{minutes} мин"),
        (0, _) => format!("{hours} ч {minutes} мин"),
        _ => format!("{days} дн {hours} ч"),
    }
}

fn memory(meminfo: &str) -> Option<String> {
    let kilobytes = |key| {
        field(meminfo, key)?
            .split_whitespace()
            .next()?
            .parse::<f64>()
            .ok()
    };
    let (total, free) = (kilobytes("MemTotal")?, kilobytes("MemAvailable")?);
    let gib = 1024.0 * 1024.0;
    Some(format!(
        "{:.1} / {:.1} ГиБ",
        (total - free) / gib,
        total / gib
    ))
}

fn count_directories(path: &str) -> usize {
    let entries = std::fs::read_dir(path).into_iter().flatten().flatten();
    entries.filter(|e| e.path().is_dir()).count()
}

fn packages() -> Option<String> {
    let dpkg = read("/var/lib/dpkg/status")
        .map_or(0, |s| s.matches("Status: install ok installed").count());
    let counts = [
        (count_directories("/var/lib/pacman/local"), "pacman"),
        (dpkg, "dpkg"),
        (count_directories("/var/lib/flatpak/app"), "flatpak"),
    ];
    let parts: Vec<_> = counts
        .iter()
        .filter(|c| c.0 > 0)
        .map(|(n, name)| format!("{n} ({name})"))
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn disk() -> Option<String> {
    let output = Command::new("df")
        .args(["-h", "--output=used,size", "/"])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let mut words = text.lines().nth(1)?.split_whitespace();
    Some(format!("{} / {}", words.next()?, words.next()?))
}

fn system() -> Vec<(&'static str, Option<String>)> {
    let os = read("/etc/os-release").and_then(|s| field(&s, "PRETTY_NAME"));
    let kernel = read("/proc/sys/kernel/osrelease").map(|s| s.trim().to_owned());
    let seconds = read("/proc/uptime").and_then(|s| s.split('.').next()?.parse().ok());
    vec![
        ("ОС", os),
        ("Ядро", kernel),
        ("Аптайм", seconds.map(uptime)),
        ("Пакеты", packages()),
    ]
}

fn hardware() -> Vec<(&'static str, Option<String>)> {
    let env = |name| std::env::var(name).ok();
    let cpu = read("/proc/cpuinfo").and_then(|s| field(&s, "model name"));
    let ram = read("/proc/meminfo").and_then(|s| memory(&s));
    vec![
        ("Shell", env("SHELL")),
        ("Окружение", env("XDG_CURRENT_DESKTOP")),
        (
            "Терминал",
            Some(format!("DimaSter {}", env!("CARGO_PKG_VERSION"))),
        ),
        ("Процессор", cpu),
        ("Память", ram),
        ("Диск /", disk()),
    ]
}

fn info_lines() -> Vec<String> {
    let user = std::env::var("USER").unwrap_or_default();
    let host = read("/proc/sys/kernel/hostname").unwrap_or_default();
    let mut lines = vec![
        format!("\x1b[1;35m{user}@{}\x1b[0m", host.trim()),
        String::new(),
    ];
    let entries = system().into_iter().chain(hardware());
    lines.extend(
        entries.filter_map(|(name, value)| Some(format!("\x1b[1;34m{name}\x1b[0m: {}", value?))),
    );
    let colors: String = (0..8).map(|c| format!("\x1b[4{c}m   ")).collect();
    lines.extend([String::new(), colors + "\x1b[0m"]);
    lines
}

// картинка не двигает курсор, текст печатается правее неё
fn print(out: &mut impl Write, logo: Option<&[u8]>) -> io::Result<()> {
    let lines = info_lines();
    if let Some(data) = logo {
        let options = format!("c={},r={},C=1", LOGO_WIDTH - 2, lines.len());
        icat::show(out, data, &options)?;
    }
    for index in 0..lines.len().max(LOGO.len()) {
        if logo.is_none() {
            let art = LOGO.get(index).copied().unwrap_or_default();
            write!(out, "\x1b[36m{art}\x1b[0m")?;
        }
        let text = lines.get(index).map(String::as_str).unwrap_or_default();
        // абсолютная колонка не зависит от ширины символов логотипа
        writeln!(out, "\x1b[{}G{text}", LOGO_WIDTH + 1)?;
    }
    Ok(())
}

/// `--logo <файл>` заменяет текстовый логотип картинкой.
pub(crate) fn run(options: &[String]) -> io::Result<()> {
    let logo = match options {
        [] => None,
        [flag, path] if flag == "--logo" => Some(std::fs::read(path)?),
        _ => {
            return Err(io::Error::other(
                "использование: dimaster fetch [--logo картинка]",
            ));
        }
    };
    print(&mut io::stdout().lock(), logo.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_system_files() {
        assert_eq!(
            field("NAME=x\nPRETTY_NAME=\"My OS\"", "PRETTY_NAME").unwrap(),
            "My OS"
        );
        let meminfo = "MemTotal:  2097152 kB\nMemAvailable:  1048576 kB";
        assert_eq!(memory(meminfo).unwrap(), "1.0 / 2.0 ГиБ");
        assert_eq!(uptime(90_000), "1 дн 1 ч");
    }
}
