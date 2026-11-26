//! Сохранение и восстановление сессий: рабочий каталог и текст истории.
//!
//! Файлы лежат в `~/.local/state/dimaster/sessions` с правами только для
//! владельца: в выводе терминала бывают секреты, поэтому всё это выключено,
//! пока пользователь не включит `session.restore`.

use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

/// Снимок одного окна.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Saved {
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) lines: Vec<String>,
}

pub(crate) fn directory() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".local/state")))?;
    Some(base.join("dimaster/sessions"))
}

fn own(directory: &Path) -> PathBuf {
    directory.join(format!("{}.session", std::process::id()))
}

/// Пишет через временный файл: обрыв не оставит половину сессии.
pub(crate) fn save(directory: &Path, saved: &Saved) -> io::Result<()> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(directory)?;
    let temporary = own(directory).with_extension("tmp");
    let mut options = std::fs::OpenOptions::new();
    let mut file = options
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)?;
    let cwd = saved
        .cwd
        .as_deref()
        .map(Path::to_string_lossy)
        .unwrap_or_default();
    writeln!(file, "{cwd}")?;
    saved
        .lines
        .iter()
        .try_for_each(|line| writeln!(file, "{line}"))?;
    std::fs::rename(temporary, own(directory))
}

pub(crate) fn forget(directory: &Path) {
    let _ = std::fs::remove_file(own(directory));
}

pub(crate) fn read(path: &Path) -> io::Result<Saved> {
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines();
    let cwd = lines.next().filter(|l| !l.is_empty()).map(PathBuf::from);
    // управляющие символы в подменённом файле не должны стать командами
    let clean = |line: &str| line.chars().filter(|c| !c.is_control()).collect();
    Ok(Saved {
        cwd,
        lines: lines.map(clean).collect(),
    })
}

/// Сессии, чьи окна уже не работают.
pub(crate) fn orphans(directory: &Path) -> Vec<PathBuf> {
    let entries = std::fs::read_dir(directory).into_iter().flatten().flatten();
    let dead = |path: &PathBuf| {
        let pid = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        path.extension().is_some_and(|e| e == "session") && !Path::new("/proc").join(pid).exists()
    };
    let mut found: Vec<_> = entries.map(|e| e.path()).filter(dead).collect();
    found.sort();
    found
}

/// Забирает сессию себе, чтобы её не восстановили дважды.
pub(crate) fn claim(path: &Path) -> Option<Saved> {
    let directory = path.parent()?;
    let mine = own(directory);
    std::fs::rename(path, &mine).ok()?;
    read(&mine).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_claim_and_forget() {
        let directory = std::env::temp_dir().join(format!("dimaster-store-{}", std::process::id()));
        let saved = Saved {
            cwd: Some("/tmp".into()),
            lines: vec!["a b".into(), "ё".into()],
        };
        save(&directory, &saved).unwrap();
        // живой процесс — не сирота
        assert!(orphans(&directory).is_empty());
        let dead = directory.join("999999999.session");
        std::fs::rename(own(&directory), &dead).unwrap();
        assert_eq!(orphans(&directory), std::slice::from_ref(&dead));
        assert_eq!(claim(&dead).unwrap(), saved);
        forget(&directory);
        assert!(!own(&directory).exists());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn strips_control_characters() {
        let path = std::env::temp_dir().join(format!("dimaster-ctl-{}", std::process::id()));
        std::fs::write(&path, "\n\x1b]0;evil\x07text\n").unwrap();
        assert_eq!(read(&path).unwrap().lines, ["]0;eviltext"]);
        std::fs::remove_file(path).unwrap();
    }
}
