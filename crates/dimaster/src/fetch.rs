//! `dimaster fetch`: сбор сведений о системе и печать макета.
//!
//! Здесь только факты из `/proc`, `/etc` и `/var/lib`; что из них показать и
//! как оформить, решает `fetch.lua` (см. `dimaster_config::run_fetch`).

use dimaster_i18n::{tr, trf};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use dimaster_config::{DEFAULT_FETCH, Layout, config_dir, run_fetch};

use crate::{fetch_random, fetch_view};

type Facts = Vec<(&'static str, Option<String>)>;

fn read(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
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
    let (d, h, n) = (days.to_string(), hours.to_string(), minutes.to_string());
    let values = [("d", d.as_str()), ("h", h.as_str()), ("n", n.as_str())];
    match (days, hours) {
        (0, 0) => trf("{n} мин", &values),
        (0, _) => trf("{h} ч {n} мин", &values),
        _ => trf("{d} дн {h} ч", &values),
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
    let used = (total - free).max(0.0);
    let pct = ((used / total.max(1.0)) * 100.0).round() as u64;
    Some(format!(
        "{:.2} / {:.2} GiB ({pct}%)",
        used / gib,
        total / gib,
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

fn output(program: &str, args: &[&str]) -> Option<String> {
    let result = Command::new(program).args(args).output().ok()?;
    String::from_utf8(result.stdout).ok()
}

fn disk() -> Option<String> {
    let text = output("df", &["-h", "--output=used,size", "/"])?;
    let mut words = text.lines().nth(1)?.split_whitespace();
    Some(format!("{} / {}", words.next()?, words.next()?))
}

// без lspci строка просто не показывается
fn gpu() -> Option<String> {
    let text = output("lspci", &[])?;
    let line = text
        .lines()
        .find(|l| l.contains("VGA") || l.contains("3D controller"))?;
    Some(line.split_once(": ")?.1.to_owned())
}

fn system() -> Facts {
    let host = read("/proc/sys/kernel/hostname").map(|s| s.trim().to_owned());
    let seconds = read("/proc/uptime").and_then(|s| s.split('.').next()?.parse().ok());
    vec![
        ("user", Some(env("USER").unwrap_or_default())),
        ("host", Some(host.unwrap_or_default())),
        (
            "os",
            read("/etc/os-release").and_then(|s| field(&s, "PRETTY_NAME")),
        ),
        (
            "kernel",
            read("/proc/sys/kernel/osrelease").map(|s| s.trim().to_owned()),
        ),
        ("uptime", seconds.map(uptime)),
        ("packages", packages()),
    ]
}

fn shell_name() -> Option<String> {
    let raw = env("SHELL")?;
    let name = Path::new(&raw)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&raw);
    Some(name.to_owned())
}

fn hardware() -> Facts {
    vec![
        ("shell", shell_name()),
        ("desktop", env("XDG_CURRENT_DESKTOP")),
        ("session", env("XDG_SESSION_TYPE")),
        (
            "terminal",
            Some(format!("DimaSter {}", env!("CARGO_PKG_VERSION"))),
        ),
        (
            "cpu",
            read("/proc/cpuinfo").and_then(|s| field(&s, "model name")),
        ),
        ("gpu", gpu()),
        ("memory", read("/proc/meminfo").and_then(|s| memory(&s))),
        ("disk", disk()),
    ]
}

// сломанный пользовательский сценарий не оставляет без вывода
fn layout() -> io::Result<Layout> {
    let facts: Facts = system().into_iter().chain(hardware()).collect();
    let directory = config_dir();
    let custom = directory
        .as_deref()
        .and_then(|d| std::fs::read_to_string(d.join("fetch.lua")).ok());
    if let Some(source) = custom {
        match run_fetch(&source, directory.as_deref(), &facts) {
            Ok(layout) => return Ok(layout),
            Err(e) => eprintln!("fetch.lua: {e}"),
        }
    }
    let mut layout =
        run_fetch(DEFAULT_FETCH, directory.as_deref(), &facts).map_err(io::Error::other)?;
    if layout.image.is_none()
        && let Some(dir) = directory.as_deref()
    {
        layout.image = ["logo.png", "logo.webp", "logo.jpg"]
            .into_iter()
            .map(|name| dir.join(name))
            .find(|p| p.is_file());
    }
    Ok(layout)
}

// без картинки слева стоит текстовый логотип
fn print(out: &mut impl Write, layout: &Layout, image: Option<&[u8]>) -> io::Result<()> {
    if let Some(data) = image {
        return fetch_view::with_picture(out, layout, data);
    }
    for index in 0..layout.lines.len().max(layout.logo.len()) {
        let logo = layout
            .logo
            .get(index)
            .map(String::as_str)
            .unwrap_or_default();
        let text = layout
            .lines
            .get(index)
            .map(String::as_str)
            .unwrap_or_default();
        // абсолютная колонка не зависит от ширины символов логотипа
        writeln!(out, "{logo}\x1b[{}G{text}", layout.logo_width + 1)?;
    }
    Ok(())
}

fn load_image(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path)
        .inspect_err(|e| {
            let values = [
                ("path", &*path.display().to_string()),
                ("error", &e.to_string()),
            ];
            eprintln!("{}", trf("логотип {path}: {error}", &values));
        })
        .ok()
}

/// `--logo <файл>` важнее картинки из сценария.
pub(crate) fn run(options: &[String]) -> io::Result<()> {
    let layout = layout()?;
    let image = match options {
        [] => {
            // случайная из папки важнее постоянной картинки
            let random = layout.random.as_deref().and_then(fetch_random::pick);
            random
                .or_else(|| layout.image.clone())
                .as_deref()
                .and_then(load_image)
        }
        [flag, path] if flag == "--logo" => Some(std::fs::read(path)?),
        _ => {
            return Err(io::Error::other(tr(
                "использование: dimaster fetch [--logo картинка]",
            )));
        }
    };
    print(&mut io::stdout().lock(), &layout, image.as_deref())
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
        assert_eq!(memory(meminfo).unwrap(), "1.00 / 2.00 GiB (50%)");
        assert_eq!(uptime(90_000), "1 дн 1 ч");
    }
}
