//! Случайная картинка из папки пользователя; подряд одна и та же не выпадает.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

fn is_picture(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
}

fn pictures(directory: &Path) -> Vec<PathBuf> {
    let entries = std::fs::read_dir(directory).into_iter().flatten().flatten();
    let mut found: Vec<_> = entries
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_picture(p))
        .collect();
    found.sort();
    found
}

// прошлая картинка исключается, если есть из чего выбирать
fn choose(mut found: Vec<PathBuf>, last: Option<&Path>, seed: u64) -> Option<PathBuf> {
    if found.len() > 1 {
        found.retain(|p| Some(p.as_path()) != last);
    }
    let index = seed.checked_rem(found.len() as u64)?;
    found.into_iter().nth(index as usize)
}

/// `$XDG_CACHE_HOME/dimaster` или `~/.cache/dimaster`.
pub(crate) fn cache_dir() -> Option<PathBuf> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".cache")))?;
    Some(cache.join("dimaster"))
}

// два окна, открытые в одну секунду, различаются наносекундами и номером процесса
fn seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    (nanos ^ u64::from(std::process::id())).wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 32
}

fn remember(file: &Path, chosen: &Path) {
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // без записи картинка всего лишь может повториться
    let _ = std::fs::write(file, chosen.as_os_str().as_encoded_bytes());
}

/// `None`, если в папке нет картинок или она недоступна.
pub(crate) fn pick(directory: &Path) -> Option<PathBuf> {
    let memory = cache_dir().map(|d| d.join("fetch-last"));
    let last = memory
        .as_ref()
        .and_then(|f| std::fs::read_to_string(f).ok());
    let chosen = choose(pictures(directory), last.as_deref().map(Path::new), seed())?;
    if let Some(file) = memory {
        remember(&file, &chosen);
    }
    Some(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_repeats_the_previous_one() {
        let two = || vec![PathBuf::from("a.png"), PathBuf::from("b.png")];
        for seed in 0..5 {
            let chosen = choose(two(), Some(Path::new("a.png")), seed);
            assert_eq!(chosen, Some(PathBuf::from("b.png")));
        }
        assert_eq!(choose(Vec::new(), None, 1), None);
        assert!(is_picture(Path::new("x.JPG")) && !is_picture(Path::new("x.txt")));
    }
}
