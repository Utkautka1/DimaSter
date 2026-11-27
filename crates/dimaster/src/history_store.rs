//! Постоянная история команд и частых директорий (аналог zoxide) в XDG_DATA_HOME.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Индекс частых директорий и сохранённых команд.
#[derive(Debug, Default)]
pub(crate) struct AssistStore {
    pub(crate) commands: Vec<String>,
    pub(crate) dirs: Vec<(u32, String)>,
}

pub(crate) fn data_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".local/share")))?;
    Some(base.join("dimaster"))
}

fn clean_line(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

impl AssistStore {
    pub(crate) fn load() -> Self {
        let Some(dir) = data_dir() else {
            return Self::default();
        };
        Self {
            commands: read_commands(&dir.join("commands.log")),
            dirs: read_dirs(&dir.join("dirs.log")),
        }
    }

    pub(crate) fn record_command(&mut self, command: &str, kept: usize) {
        let cmd = clean_line(command.trim());
        if cmd.is_empty() {
            return;
        }
        self.commands.retain(|c| c != &cmd);
        self.commands.push(cmd);
        if self.commands.len() > kept {
            let drain = self.commands.len() - kept;
            self.commands.drain(..drain);
        }
        if let Some(dir) = data_dir() {
            let _ = write_lines(&dir.join("commands.log"), &self.commands);
        }
    }

    pub(crate) fn bump_dir(&mut self, path: &Path, kept: usize) {
        let raw = clean_line(&path.to_string_lossy());
        if raw.is_empty() {
            return;
        }
        let prev = self
            .dirs
            .iter()
            .position(|(_, p)| p == &raw)
            .map_or(0, |i| self.dirs.remove(i).0);
        self.dirs.push((prev.saturating_add(1), raw));
        self.dirs.sort_by_key(|a| std::cmp::Reverse(a.0));
        self.dirs.truncate(kept);
        if let Some(dir) = data_dir() {
            let lines: Vec<String> = self.dirs.iter().map(|(s, p)| format!("{s}\t{p}")).collect();
            let _ = write_lines(&dir.join("dirs.log"), &lines);
        }
    }

    /// Команды, начинающиеся с `prefix`: сначала новые, без повторов.
    pub(crate) fn matching(&self, prefix: &str, live: &[String]) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        let all = self.commands.iter().chain(live).rev();
        for command in all.filter(|c| c.starts_with(prefix) && c.as_str() != prefix) {
            if !found.contains(command) {
                found.push(command.clone());
            }
        }
        found
    }

    pub(crate) fn best_suggestion(&self, prefix: &str, live: &[String]) -> Option<String> {
        if prefix.is_empty() {
            return None;
        }
        live.iter()
            .rev()
            .chain(self.commands.iter().rev())
            .find(|cmd| cmd.len() > prefix.len() && cmd.starts_with(prefix))
            .map(|cmd| cmd[prefix.len()..].to_owned())
    }
}

fn read_commands(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|s| {
            s.lines()
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn read_dirs(path: &Path) -> Vec<(u32, String)> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out: Vec<(u32, String)> = text
        .lines()
        .filter_map(|line| {
            let (score, dir) = line.split_once('\t')?;
            Some((score.parse::<u32>().ok()?, dir.to_owned()))
        })
        .collect();
    out.sort_by_key(|a| std::cmp::Reverse(a.0));
    out
}

fn write_lines(path: &Path, lines: &[String]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    let mut file = std::fs::File::create(&tmp)?;
    for line in lines {
        writeln!(file, "{line}")?;
    }
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_most_recent_matching_suggestion() {
        let store = AssistStore {
            commands: vec!["git status".into(), "grep foo".into()],
            ..Default::default()
        };
        let live = vec!["git switch main".into()];
        assert_eq!(
            store.best_suggestion("git s", &live).as_deref(),
            Some("witch main")
        );
        assert_eq!(
            store.best_suggestion("git st", &live).as_deref(),
            Some("atus")
        );
        assert!(store.best_suggestion("git switch main", &live).is_none());
    }
}
