//! История самого shell: без неё стрелки знали бы только команды из DimaSter.
//!
//! Файлы только читаются. zsh пишет байты вне ASCII «метафицированными»:
//! `0x83`, затем байт с перевёрнутым пятым битом.

use std::path::PathBuf;

const META: u8 = 0x83;

fn unmeta(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut escaped = false;
    for &byte in bytes {
        match (escaped, byte) {
            (false, META) => escaped = true,
            (true, _) => {
                out.push(byte ^ 32);
                escaped = false;
            }
            _ => out.push(byte),
        }
    }
    out
}

// `: время:длительность;команда` у zsh, `- cmd: команда` у fish
fn command(line: &str) -> Option<&str> {
    let stamped = line.strip_prefix(": ").and_then(|l| l.split_once(';'));
    let text = match (stamped, line.strip_prefix("- cmd: ")) {
        (Some((_, command)), _) | (None, Some(command)) => command,
        // служебные строки fish и отметки времени bash
        _ if line.starts_with("  ") || line.starts_with('#') => return None,
        _ => line,
    };
    // продолжения многострочных команд одной стрелкой не вставить
    let whole = !text.is_empty() && !text.ends_with('\\');
    (whole && !text.chars().any(char::is_control)).then_some(text)
}

fn parse(bytes: &[u8], zsh: bool) -> Vec<String> {
    let bytes = if zsh { unmeta(bytes) } else { bytes.to_vec() };
    let text = String::from_utf8_lossy(&bytes);
    text.lines()
        .filter_map(command)
        .map(str::to_owned)
        .collect()
}

fn path(shell: &str) -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    Some(match shell {
        "zsh" => std::env::var_os("HISTFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".zsh_history")),
        "bash" => home.join(".bash_history"),
        "fish" => home.join(".local/share/fish/fish_history"),
        _ => return None,
    })
}

/// Команды из истории shell, от старых к новым.
pub(crate) fn read(program: &str) -> Vec<String> {
    let shell = program.rsplit('/').next().unwrap_or_default();
    let found = path(shell).and_then(|p| std::fs::read(p).ok());
    found.map_or_else(Vec::new, |bytes| parse(&bytes, shell == "zsh"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_zsh_bash_and_fish_lines() {
        let zsh = b": 1700000000:0;git push\nls -la\n: 1:0;multi \\\n";
        assert_eq!(parse(zsh, true), ["git push", "ls -la"]);
        let fish = b"- cmd: git log\n  when: 1700000000\n#1700\n";
        assert_eq!(parse(fish, false), ["git log"]);
        // «ы» (d1 8b) в файле zsh: второй байт экранирован
        assert_eq!(parse(&[0xd1, META, 0x8b ^ 32], true), ["ы"]);
    }
}
