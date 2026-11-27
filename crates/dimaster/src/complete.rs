//! Что предложить по `Tab`: команды, флаги из `man` и файлы.
//!
//! Программа, чьи флаги нужны, не запускается: читается только её `man`.
//! Если предложить нечего, возвращается `None` и дополняет сам shell.

use std::path::{Path, PathBuf};
use std::process::Command;

use dimaster_docker::capture;

use crate::modes::picker::shell_escape;
use crate::view::View;

/// Дополняемое слово и чем его можно заменить.
pub(crate) struct Found {
    pub(crate) word: String,
    pub(crate) items: Vec<String>,
}

/// Стирает набранное слово и вставляет выбранное; после каталога без пробела.
pub(crate) fn replace(word: &str, choice: &str) -> Vec<u8> {
    let mut bytes = vec![0x7f; word.chars().count()];
    bytes.extend_from_slice(choice.as_bytes());
    if !choice.trim_end_matches('\'').ends_with('/') {
        bytes.push(b' ');
    }
    bytes
}

// `~/` и относительные пути считаются от дома и каталога shell
fn resolve(directory: &str, cwd: &Path) -> PathBuf {
    match (directory.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => cwd.join(directory),
    }
}

// набранная часть пути остаётся как есть, экранируется только имя
fn files(word: &str, cwd: &Path, only_directories: bool, limit: usize) -> Vec<String> {
    let split = word.rfind('/').map_or(0, |i| i + 1);
    let (directory, start) = word.split_at(split);
    let Ok(entries) = std::fs::read_dir(resolve(directory, cwd)) else {
        return Vec::new();
    };
    let mut found: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let is_directory = e.path().is_dir();
            let name = e.file_name().into_string().ok()?;
            let hidden = name.starts_with('.') && !start.starts_with('.');
            let fits = name.starts_with(start) && !name.chars().any(char::is_control);
            (fits && !hidden && (is_directory || !only_directories)).then(|| {
                let slash = if is_directory { "/" } else { "" };
                format!("{directory}{}{slash}", shell_escape(&name))
            })
        })
        .take(limit)
        .collect();
    found.sort();
    found
}

// строки вида `-a, --all` из раздела с описанием параметров
fn flags_in(manual: &str) -> Vec<String> {
    let mut found: Vec<String> = manual
        .lines()
        .map(str::trim_start)
        .filter(|l| l.starts_with('-'))
        .flat_map(|l| l.split([' ', ',', '=', '[', '<', '|']))
        .filter(|w| w.starts_with('-') && !w.trim_start_matches('-').is_empty())
        .filter(|w| w.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .map(str::to_owned)
        .collect();
    found.sort();
    found.dedup();
    found
}

fn manual_flags(program: &str, start: &str) -> Vec<String> {
    let plain = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.');
    if program.starts_with('-') || !program.chars().all(plain) {
        return Vec::new();
    }
    let mut command = Command::new("man");
    command.args(["-P", "cat", program]);
    command.env("MANWIDTH", "100");
    let wait = dimaster_limits::SENSOR_TIMEOUT.secs();
    let text = capture(command, wait).unwrap_or_default();
    let mut flags = flags_in(&text);
    flags.retain(|f| f.starts_with(start));
    flags
}

fn arguments(view: &View, program: &str, word: &str) -> Vec<String> {
    let native = &view.config.native;
    if word.starts_with('-') {
        return match native.man_flags {
            true => manual_flags(program, word),
            false => Vec::new(),
        };
    }
    let listed = native.path_commands.iter().any(|c| c == program);
    if !(listed || word.contains('/')) {
        return Vec::new();
    }
    let cwd = view.shell_directory().unwrap_or_else(|| ".".into());
    files(word, &cwd, program == "cd", view.config.tuning.picker_files)
}

/// Варианты для набранной строки.
pub(crate) fn find(view: &mut View, input: &str, spaced: bool) -> Option<Found> {
    // кавычки и экранирование разбирает только сам shell
    if input.contains(['\'', '"', '\\']) {
        return None;
    }
    let word = match spaced {
        true => "",
        false => input.split_whitespace().next_back()?,
    };
    let program = input.split_whitespace().next()?;
    let first = !spaced && program == word && !word.contains('/');
    let items = match first {
        true => view.syntax.names(word),
        false => arguments(view, program, word),
    };
    // единственный вариант, равный набранному, — решать shell
    let useless = items.is_empty() || items == [word];
    (!useless).then(|| Found {
        word: word.to_owned(),
        items,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_flags_from_manual_text() {
        let manual = "  -a, --all\n         text -x\n  --color[=WHEN]\n - item";
        assert_eq!(flags_in(manual), ["--all", "--color", "-a"]);
    }

    #[test]
    fn completes_files_and_keeps_the_typed_directory() {
        let root = std::env::temp_dir().join(format!("dimaster-tab-{}", std::process::id()));
        std::fs::create_dir_all(root.join("sub/my dir")).expect("каталог");
        std::fs::write(root.join("sub/my.txt"), "").expect("файл");
        assert_eq!(
            files("sub/my", &root, false, 10),
            ["sub/'my dir'/", "sub/my.txt"]
        );
        assert_eq!(files("sub/my", &root, true, 10), ["sub/'my dir'/"]);
        assert_eq!(replace("my", "my.txt"), b"\x7f\x7fmy.txt ");
        assert_eq!(replace("", "'a b'/"), b"'a b'/");
        std::fs::remove_dir_all(root).expect("уборка");
    }
}
