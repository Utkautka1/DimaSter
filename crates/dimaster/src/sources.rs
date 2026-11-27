//! Откуда берутся строки списков: серверы из `~/.ssh/config` и контейнеры.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

// вложенные Include глубже этого — почти наверняка цикл
const MAX_INCLUDE: u8 = 3;
// зависший демон Docker не должен надолго замораживать окно
const DOCKER_TIMEOUT: Duration = Duration::from_secs(2);

/// Имя уходит в командную строку: никаких пробелов, кавычек и ведущего `-`.
pub(crate) fn safe_name(name: &str) -> bool {
    let allowed = |c: char| c.is_ascii_alphanumeric() || "._-@:".contains(c);
    !name.is_empty() && !name.starts_with('-') && name.chars().all(allowed)
}

// `Include` понимает `~/`, пути от ~/.ssh и `*` в имени файла
fn included(pattern: &str, ssh: &Path) -> Vec<PathBuf> {
    let path = match pattern.strip_prefix("~/") {
        Some(rest) => ssh.parent().unwrap_or(ssh).join(rest),
        None => ssh.join(pattern),
    };
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let Some((head, tail)) = name.split_once('*') else {
        return vec![path];
    };
    let entries = std::fs::read_dir(path.parent().unwrap_or(ssh));
    let mut found: Vec<PathBuf> = entries
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            let file = e.file_name().to_string_lossy().into_owned();
            file.starts_with(head) && file.ends_with(tail)
        })
        .map(|e| e.path())
        .collect();
    found.sort();
    found
}

fn collect_hosts(file: &Path, ssh: &Path, depth: u8, out: &mut Vec<String>) {
    let Ok(text) = std::fs::read_to_string(file) else {
        return;
    };
    for line in text.lines() {
        let mut words = line
            .split(|c: char| c.is_whitespace() || c == '=')
            .filter(|w| !w.is_empty());
        let Some(keyword) = words.next() else {
            continue;
        };
        if keyword.eq_ignore_ascii_case("host") {
            // шаблоны с * ? ! — это настройки, а не серверы
            let names = words.filter(|w| safe_name(w) && !out.iter().any(|o| o == w));
            let names: Vec<String> = names.map(str::to_owned).collect();
            out.extend(names);
        } else if keyword.eq_ignore_ascii_case("include") && depth < MAX_INCLUDE {
            let files = words.flat_map(|w| included(w, ssh));
            files.for_each(|f| collect_hosts(&f, ssh, depth + 1, out));
        }
    }
}

/// Серверы из `~/.ssh/config` и подключённых им файлов.
pub(crate) fn ssh_hosts() -> Vec<String> {
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let ssh = PathBuf::from(home).join(".ssh");
    let mut out = Vec::new();
    collect_hosts(&ssh.join("config"), &ssh, 0, &mut out);
    out
}

/// Строка списка и имя контейнера; `up` — запущен ли он.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Container {
    pub(crate) name: String,
    pub(crate) up: bool,
    pub(crate) line: String,
}

fn parse_containers(output: &str) -> Vec<Container> {
    let rows = output.lines().filter_map(|row| {
        let mut parts = row.split('\t');
        let (name, status, image) = (parts.next()?, parts.next()?, parts.next()?);
        let up = status.starts_with("Up");
        let mark = if up { '▶' } else { '■' };
        safe_name(name).then(|| Container {
            name: name.to_owned(),
            up,
            line: format!("{mark} {name}  {status}  {image}"),
        })
    });
    rows.collect()
}

/// `None`, если Docker не установлен, не отвечает или отказал.
pub(crate) fn containers() -> Option<Vec<Container>> {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let format = "{{.Names}}\t{{.Status}}\t{{.Image}}";
        let output = Command::new("docker")
            .args(["ps", "--all", "--format", format])
            .stdin(Stdio::null())
            .output();
        // получатель мог уже перестать ждать
        let _ = send.send(output);
    });
    let output = receive.recv_timeout(DOCKER_TIMEOUT).ok()?.ok()?;
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    output.status.success().then(|| parse_containers(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_skip_patterns_and_follow_includes() {
        let root = std::env::temp_dir().join(format!("dimaster-ssh-{}", std::process::id()));
        std::fs::create_dir_all(root.join(".ssh/conf.d")).unwrap();
        let main = "Include conf.d/*.conf\nHost web db\n  HostName x\nHost *\nHost=-bad\n";
        std::fs::write(root.join(".ssh/config"), main).unwrap();
        std::fs::write(root.join(".ssh/conf.d/a.conf"), "host jump web\n").unwrap();
        let mut hosts = Vec::new();
        let ssh = root.join(".ssh");
        collect_hosts(&ssh.join("config"), &ssh, 0, &mut hosts);
        std::fs::remove_dir_all(root).unwrap();
        assert_eq!(hosts, ["jump", "web", "db"]);
    }

    #[test]
    fn containers_are_parsed_and_unsafe_names_dropped() {
        let found = parse_containers("api\tUp 2 hours\tnginx\nold\tExited (0)\tredis\na b\tUp\tx");
        let states: Vec<_> = found.iter().map(|c| (c.name.as_str(), c.up)).collect();
        assert_eq!(states, [("api", true), ("old", false)]);
    }
}
