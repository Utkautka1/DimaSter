//! Данные для виджетов. Собираются в отдельном потоке: git и docker бывают
//! медленными и не должны задерживать окно.
//!
//! Команды запускаются в чужих каталогах, поэтому у git отключены fsmonitor и
//! хуки из конфига репозитория, а версии инструментов спрашиваются из `/`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// тактов в секунде в /proc/<pid>/stat на всех распространённых ядрах
const TICKS: f64 = 100.0;

pub(crate) type Facts = Vec<(String, String)>;

/// Что нужно знать потоку сбора; обновляется окном.
#[derive(Debug, Default, Clone)]
pub(crate) struct Target {
    pub(crate) cwd: Option<PathBuf>,
    // номер процесса shell; 0 — ещё не запущен
    pub(crate) shell: u32,
    pub(crate) docker: bool,
}

#[derive(Debug, PartialEq)]
struct Process {
    name: String,
    group: i32,
    foreground: i32,
    ticks: u64,
    // страницы по 4 КиБ
    pages: u64,
}

// имя в скобках может содержать пробелы, поэтому режем по последней скобке
fn parse_stat(stat: &str) -> Option<Process> {
    let (head, tail) = stat.rsplit_once(')')?;
    let name = head.split_once('(')?.1.to_owned();
    let fields: Vec<&str> = tail.split_whitespace().collect();
    let number = |i: usize| fields.get(i)?.parse::<i64>().ok();
    Some(Process {
        name,
        group: number(2)? as i32,
        foreground: number(5)? as i32,
        ticks: (number(11)? + number(12)?) as u64,
        pages: number(21)? as u64,
    })
}

fn process(pid: &str) -> Option<Process> {
    parse_stat(&std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)
}

// процессы группы, занимающей терминал
fn foreground(shell: u32) -> Option<(String, u64, u64)> {
    let own = process(&shell.to_string())?;
    if own.foreground == own.group || own.foreground <= 0 {
        return None;
    }
    let entries = std::fs::read_dir("/proc").ok()?.flatten();
    let members = entries
        .filter_map(|e| process(e.file_name().to_str()?))
        .filter(|p| p.group == own.foreground);
    let mut total: Option<(String, u64, u64)> = None;
    for member in members {
        let sum = total.get_or_insert((member.name, 0, 0));
        sum.1 += member.ticks;
        sum.2 += member.pages;
    }
    total
}

fn output(program: &str, args: &[&str], directory: &Path) -> Option<String> {
    let result = Command::new(program)
        .args(args)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    result
        .status
        .success()
        .then(|| String::from_utf8_lossy(&result.stdout).into_owned())
}

// первая строка `status -b` — ветка, остальные — изменённые файлы
fn parse_git(text: &str) -> Option<(String, usize)> {
    let mut lines = text.lines();
    let head = lines.next()?.strip_prefix("## ")?;
    let branch = head.strip_prefix("No commits yet on ").unwrap_or(head);
    let branch = branch.split("...").next()?.to_owned();
    Some((branch, lines.count()))
}

fn git(directory: &Path) -> Option<(String, usize)> {
    const SAFE: [&str; 5] = [
        "--no-optional-locks",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "core.hooksPath=/dev/null",
    ];
    let args = [&SAFE[..], &["status", "--porcelain=v1", "-b"]].concat();
    parse_git(&output("git", &args, directory)?)
}

// проект на этом языке — если в каталоге есть его файл
const TOOLS: [(&str, &[&str], &str, &str); 3] = [
    ("node", &["package.json"], "node", "--version"),
    (
        "python",
        &["pyproject.toml", "requirements.txt", "setup.py"],
        "python3",
        "--version",
    ),
    ("go", &["go.mod"], "go", "version"),
];

// «v22.1.0», «Python 3.13.1», «go version go1.24 linux/amd64»
fn version_number(text: &str) -> Option<String> {
    let word = text.split_whitespace().find(|w| w.contains('.'))?;
    Some(
        word.trim_start_matches(|c: char| !c.is_ascii_digit())
            .to_owned(),
    )
}

/// Состояние между опросами: загрузка CPU считается по разнице.
#[derive(Debug, Default)]
struct Sampler {
    versions: HashMap<&'static str, Option<String>>,
    previous: Option<(String, u64, Instant)>,
}

impl Sampler {
    fn usage(&mut self, shell: u32, facts: &mut Facts) {
        let Some((name, ticks, pages)) = foreground(shell) else {
            self.previous = None;
            return;
        };
        let now = Instant::now();
        let cpu = self
            .previous
            .take()
            .filter(|p| p.0 == name)
            .map_or(0.0, |(_, before, at)| {
                let busy = ticks.saturating_sub(before) as f64 / TICKS;
                busy / now.duration_since(at).as_secs_f64().max(0.1) * 100.0
            });
        facts.push(("process".into(), name.clone()));
        facts.push(("cpu".into(), format!("{cpu:.0}")));
        facts.push(("memory".into(), format!("{} МБ", pages * 4 / 1024)));
        self.previous = Some((name, ticks, now));
    }

    fn project(&mut self, directory: &Path, facts: &mut Facts) {
        if let Some((branch, changes)) = git(directory) {
            facts.push(("git_branch".into(), branch));
            if changes > 0 {
                facts.push(("git_changes".into(), changes.to_string()));
            }
        }
        for (name, markers, program, flag) in TOOLS {
            if !markers.iter().any(|m| directory.join(m).exists()) {
                continue;
            }
            let version = self
                .versions
                .entry(name)
                .or_insert_with(|| version_number(&output(program, &[flag], Path::new("/"))?));
            facts.extend(version.clone().map(|v| (name.into(), v)));
        }
    }

    fn sample(&mut self, target: &Target) -> Facts {
        let mut facts = Facts::new();
        if let Some(directory) = &target.cwd {
            facts.push(("cwd".into(), directory.display().to_string()));
            self.project(directory, &mut facts);
        }
        if target.shell != 0 {
            self.usage(target.shell, &mut facts);
        }
        if target.docker {
            docker(&mut facts);
        }
        facts
    }
}

fn docker(facts: &mut Facts) {
    let args = ["ps", "-a", "--format", "{{.State}}"];
    let Some(text) = output("docker", &args, Path::new("/")) else {
        return;
    };
    let running = text.lines().filter(|l| *l == "running").count();
    facts.push(("docker_running".into(), running.to_string()));
    facts.push((
        "docker_stopped".into(),
        (text.lines().count() - running).to_string(),
    ));
}

/// Запускает опрос; поток завершается, когда `deliver` возвращает `false`.
pub(crate) fn start(
    interval: Duration,
    target: Arc<Mutex<Target>>,
    deliver: impl Fn(Facts) -> bool + Send + 'static,
) {
    thread::spawn(move || {
        let mut sampler = Sampler::default();
        loop {
            let current = target.lock().map(|t| t.clone()).unwrap_or_default();
            if !deliver(sampler.sample(&current)) {
                return;
            }
            thread::sleep(interval);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stat_git_and_versions() {
        let stat = "7 (a b) S 1 7 7 34816 9 0 0 0 0 0 3 4 0 0 20 0 1 0 5 1000 256 0";
        let parsed = parse_stat(stat).unwrap();
        assert_eq!(
            (parsed.name.as_str(), parsed.group, parsed.foreground),
            ("a b", 7, 9)
        );
        assert_eq!((parsed.ticks, parsed.pages), (7, 256));
        let git = parse_git("## main...origin/main [ahead 1]\n M a\n?? b\n").unwrap();
        assert_eq!(git, ("main".to_owned(), 2));
        assert_eq!(
            version_number("go version go1.24.1 linux/amd64").unwrap(),
            "1.24.1"
        );
        assert_eq!(version_number("v22.1.0\n").unwrap(), "22.1.0");
    }
}
