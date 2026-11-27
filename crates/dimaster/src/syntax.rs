//! Встроенная подсветка вводимых команд и проверка их существования в `$PATH`.
//!
//! Работает напрямую при отрисовке кадра для любой оболочки (`zsh`, `bash`,
//! `fish`, `sh`), не требуя внешних ZSH-плагинов и не затирая уже заданные цвета.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use dimaster_grid::{Color, Flags, Match, Point, PromptMark, Span, Terminal};

// Как часто перечитывать каталоги `$PATH`, чтобы не делать лишних stat при наборе текста.
const PATH_TTL: Duration = Duration::from_secs(5);

// Встроенные команды POSIX/bash/zsh/fish (отсортировано для binary_search).
const BUILTINS: &[&str] = &[
    ".",
    "[",
    "[[",
    "alias",
    "autoload",
    "bg",
    "bind",
    "bindkey",
    "break",
    "builtin",
    "case",
    "cd",
    "clear",
    "command",
    "compdef",
    "continue",
    "declare",
    "dirs",
    "do",
    "done",
    "echo",
    "elif",
    "else",
    "emulate",
    "esac",
    "eval",
    "exec",
    "exit",
    "export",
    "false",
    "fg",
    "fi",
    "for",
    "functions",
    "getopts",
    "hash",
    "help",
    "history",
    "if",
    "in",
    "jobs",
    "kill",
    "local",
    "nocorrect",
    "noglob",
    "popd",
    "printf",
    "pushd",
    "pwd",
    "read",
    "return",
    "select",
    "set",
    "setopt",
    "shift",
    "source",
    "test",
    "then",
    "time",
    "trap",
    "true",
    "type",
    "typeset",
    "ulimit",
    "umask",
    "unalias",
    "unset",
    "unsetopt",
    "until",
    "wait",
    "whence",
    "which",
    "while",
    "zstyle",
];

// Команды-обёртки, после которых следующее слово всё ещё является именем команды.
const WRAPPERS: &[&str] = &[
    "builtin", "command", "doas", "env", "exec", "nohup", "sudo", "time", "watch",
];

// Стандартная палитра LS_COLORS, если в окружении пользователя её ещё нет.
pub(crate) const DEFAULT_LS_COLORS: &str = "di=1;34:ln=36:so=35:pi=33:ex=1;32:bd=33;1:cd=33;1:or=31;1:*.tar=31:*.tgz=31:*.zip=31:*.zst=31:*.gz=31:*.xz=31:*.7z=31:*.png=35:*.jpg=35:*.jpeg=35:*.webp=35:*.svg=35:*.gif=35:*.mp4=35:*.mp3=36:*.flac=36:*.rs=33:*.py=33:*.lua=34:*.sh=32:*.toml=36:*.json=36:*.yaml=36:*.yml=36:*.md=37";

#[derive(Debug, Clone, Copy)]
struct CellChar {
    line: u64,
    col: usize,
    ch: char,
}

/// Кэш исполняемых файлов из `$PATH` для мгновенной проверки при печати.
#[derive(Debug, Default)]
pub(crate) struct CommandCache {
    bins: HashSet<String>,
    updated: Option<Instant>,
    last_path: String,
}

impl CommandCache {
    fn refresh(&mut self) {
        let path = std::env::var("PATH").unwrap_or_default();
        let fresh = self
            .updated
            .is_some_and(|t| t.elapsed() < PATH_TTL && path == self.last_path);
        if fresh {
            return;
        }
        self.bins.clear();
        for dir in std::env::split_paths(&path) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        self.bins.insert(name.to_owned());
                    }
                }
            }
        }
        self.last_path = path;
        self.updated = Some(Instant::now());
    }

    fn exists(&mut self, word: &str, cwd: Option<&str>) -> bool {
        if BUILTINS.binary_search(&word).is_ok() {
            return true;
        }
        if word.contains('/') || word.starts_with("~/") {
            return path_exists(word, cwd);
        }
        self.refresh();
        self.bins.contains(word)
    }

    /// Ищет наиболее близкую команду по расстоянию Левенштейна ≤ 2 (аналог thefuck).
    pub(crate) fn correction(&mut self, word: &str) -> Option<String> {
        if word.len() < 2 || word.contains('/') {
            return None;
        }
        self.refresh();
        let candidates = BUILTINS
            .iter()
            .copied()
            .chain(self.bins.iter().map(String::as_str));
        let mut best: Option<(usize, &str)> = None;
        for cand in candidates {
            if cand.len().abs_diff(word.len()) > 2 || cand == word {
                continue;
            }
            if let Some(d) = edit_distance(word, cand, 2)
                && best.is_none_or(|(bd, bc)| (d, cand.len()) < (bd, bc.len()))
            {
                best = Some((d, cand));
                if d == 1 && cand.len() == word.len() {
                    break;
                }
            }
        }
        best.map(|(_, c)| c.to_owned())
    }
}

fn edit_distance(a: &str, b: &str, limit: usize) -> Option<usize> {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > limit {
        return None;
    }
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let next = (row[j + 1] + 1).min(row[j] + 1).min(prev + cost);
            prev = row[j + 1];
            row[j + 1] = next;
        }
    }
    (row[b.len()] <= limit).then_some(row[b.len()])
}

fn path_exists(word: &str, cwd: Option<&str>) -> bool {
    let path = if let Some(rest) = word.strip_prefix("~/") {
        let Some(home) = std::env::var_os("HOME") else {
            return false;
        };
        PathBuf::from(home).join(rest)
    } else {
        let p = Path::new(word);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            Path::new(cwd.unwrap_or(".")).join(p)
        }
    };
    path.is_file()
}

/// Проверяет, находится ли курсор внутри редактируемой строки приглашения shell.
pub(crate) fn editing_command(term: &Terminal) -> bool {
    let cursor_line = term.visible_line(0) + term.cursor().row as u64;
    term.input_start().is_some_and(|(line, col)| {
        line <= cursor_line && (line < cursor_line || term.cursor().col >= col)
    }) || fallback_start(term).is_some()
}

/// Возвращает набранную команду, если курсор стоит в самом конце ввода.
// под строкой ввода что-то есть — открыт список дополнения: стрелки и место
// принадлежат ему, а не подсказке
fn list_below(term: &Terminal) -> bool {
    let below = term.screen().rows().iter().skip(term.cursor().row + 1);
    below
        .take(2)
        .any(|row| !row.wrapped && row.text().chars().any(|c| c != ' '))
}

pub(crate) fn input_text_at_end(term: &Terminal) -> Option<String> {
    let cells = read_input_cells(term)?;
    let last = cells.last()?;
    let cursor = term.cursor();
    let cursor_line = term.visible_line(0) + cursor.row as u64;
    // чужая подсказка под курсором уже занимает место нашей
    let below = term
        .line(cursor_line)
        .and_then(|r| r.cells.get(cursor.col).map(|c| c.ch));
    let taken = below.is_some_and(|ch| ch != ' ');
    if cursor_line != last.line || cursor.col <= last.col || taken || list_below(term) {
        return None;
    }
    Some(cells.iter().map(|c| c.ch).collect())
}

/// Если введено одно ненайденное слово (опечатка), возвращает `(опечатка, исправление)`.
pub(crate) fn suggest_typo(
    text: &str,
    cwd: Option<&str>,
    cache: &mut CommandCache,
) -> Option<(String, String)> {
    let word = text.trim();
    if word.is_empty() || word.contains(char::is_whitespace) || cache.exists(word, cwd) {
        return None;
    }
    cache.correction(word).map(|fix| (word.to_owned(), fix))
}

/// Собирает подсветку синтаксиса для текущей редактируемой строки команды.
pub(crate) fn highlight_input(
    term: &Terminal,
    palette: &[u32; 16],
    cache: &mut CommandCache,
) -> Vec<(Span, u32)> {
    let Some(cells) = read_input_cells(term) else {
        return Vec::new();
    };
    tokenize(&cells, palette, term.cwd(), cache)
        .into_iter()
        .map(|(m, color)| (term.match_span(m), color))
        .collect()
}

fn read_input_cells(term: &Terminal) -> Option<Vec<CellChar>> {
    let cursor_line = term.visible_line(0) + term.cursor().row as u64;
    let (start_line, start_col) = term
        .input_start()
        .filter(|&(line, _)| line <= cursor_line)
        .or_else(|| fallback_start(term))?;
    let range = term.line_range();
    if start_line < range.start || start_line >= range.end {
        return None;
    }
    let cursor = (cursor_line, term.cursor().col);
    let mut out = collect_wrapped_cells(term, (start_line, start_col), range.end, cursor);
    trim_trailing_spaces(&mut out);
    (!out.is_empty()).then_some(out)
}

// `cursor` — (строка, колонка): левее курсора всё набрано пользователем
fn collect_wrapped_cells(
    term: &Terminal,
    (start_line, start_col): (u64, usize),
    end_line: u64,
    cursor: (u64, usize),
) -> Vec<CellChar> {
    let mut out = Vec::new();
    let mut col_from = start_col;
    for line in start_line..end_line {
        let Some(row) = term.line(line) else { break };
        if line > start_line && !row.wrapped {
            break;
        }
        for (col, cell) in row.cells.iter().enumerate().skip(col_from) {
            if cell.style.flags.contains(Flags::WIDE_TAIL) {
                continue;
            }
            // Цветное правее курсора — RPROMPT или подсказка плагина, а не ввод.
            // Левее цвет не важен: набранное красит zsh-syntax-highlighting.
            if cell.style.fg != Color::Default && (line, col) >= cursor && !out.is_empty() {
                return out;
            }
            out.push(CellChar {
                line,
                col,
                ch: cell.ch,
            });
        }
        col_from = 0;
    }
    out
}

fn trim_trailing_spaces(cells: &mut Vec<CellChar>) {
    while cells.last().is_some_and(|c| c.ch == ' ') {
        cells.pop();
    }
}

// Запасной поиск начала ввода для оболочек без OSC 133;B (поддерживает многострочный prompt).
fn fallback_start(term: &Terminal) -> Option<(u64, usize)> {
    let cursor = term.cursor();
    let cursor_line = term.visible_line(0) + cursor.row as u64;
    let has_prompt = (cursor_line.saturating_sub(3)..=cursor_line).any(|l| {
        term.line(l)
            .and_then(|r| r.mark)
            .is_some_and(|m| matches!(m, PromptMark::Prompt | PromptMark::Failed))
    });
    let row = has_prompt.then(|| term.line(cursor_line)).flatten()?;
    let limit = cursor.col.min(row.cells.len());
    for i in (0..limit).rev() {
        let ch = row.cells[i].ch;
        if matches!(ch, '➜' | '❯' | '→' | '$' | '%' | '#' | '>')
            && row.cells.get(i + 1).is_some_and(|c| c.ch == ' ')
        {
            return Some((cursor_line, i + 2));
        }
    }
    None
}

fn tokenize(
    cells: &[CellChar],
    palette: &[u32; 16],
    cwd: Option<&str>,
    cache: &mut CommandCache,
) -> Vec<(Match, u32)> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut expect_cmd = true;
    while i < cells.len() {
        i = step_token(cells, i, &mut expect_cmd, palette, cwd, cache, &mut out);
    }
    out
}

fn step_token(
    cells: &[CellChar],
    i: usize,
    expect_cmd: &mut bool,
    palette: &[u32; 16],
    cwd: Option<&str>,
    cache: &mut CommandCache,
    out: &mut Vec<(Match, u32)>,
) -> usize {
    if cells[i].ch == ' ' {
        return i + 1;
    }
    if cells[i].ch == '#' {
        push_span(out, &cells[i..], palette[8]);
        return cells.len();
    }
    if let Some((end, resets)) = take_operator(cells, i) {
        push_span(out, &cells[i..end], palette[5]);
        *expect_cmd = resets;
        return end;
    }
    if matches!(cells[i].ch, '\'' | '"') {
        let end = take_quoted(cells, i);
        push_span(out, &cells[i..end], palette[3]);
        *expect_cmd = false;
        return end;
    }
    let end = take_word(cells, i);
    classify_word(&cells[i..end], expect_cmd, palette, cwd, cache, out);
    end
}

fn classify_word(
    slice: &[CellChar],
    expect_cmd: &mut bool,
    palette: &[u32; 16],
    cwd: Option<&str>,
    cache: &mut CommandCache,
    out: &mut Vec<(Match, u32)>,
) {
    let word: String = slice.iter().map(|c| c.ch).collect();
    if *expect_cmd {
        if let Some(eq) = word.find('=').filter(|&p| p > 0 && is_env_name(&word[..p])) {
            push_span(out, &slice[..=eq], palette[6]);
            if eq + 1 < slice.len() {
                push_span(out, &slice[eq + 1..], palette[3]);
            }
            return;
        }
        if WRAPPERS.contains(&word.as_str()) {
            push_span(out, slice, palette[2]);
            return;
        }
        let color = if cache.exists(&word, cwd) {
            palette[2]
        } else {
            palette[1]
        };
        push_span(out, slice, color);
        *expect_cmd = false;
    } else if word.starts_with('$') {
        push_span(out, slice, palette[6]);
    } else if word.starts_with('-') && word.len() > 1 {
        push_span(out, slice, palette[4]);
    }
}

fn is_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn take_operator(cells: &[CellChar], start: usize) -> Option<(usize, bool)> {
    let c = cells[start].ch;
    let next = cells.get(start + 1).map(|n| n.ch);
    match c {
        '|' | '&' if next == Some(c) => Some((start + 2, true)),
        '|' | ';' | '&' | '(' => Some((start + 1, true)),
        '>' | '<' if next == Some(c) => Some((start + 2, false)),
        '>' | '<' | ')' => Some((start + 1, false)),
        _ => None,
    }
}

fn take_quoted(cells: &[CellChar], start: usize) -> usize {
    let quote = cells[start].ch;
    let mut i = start + 1;
    let mut escaped = false;
    while i < cells.len() {
        let ch = cells[i].ch;
        i += 1;
        if escaped {
            escaped = false;
        } else if ch == '\\' && quote == '"' {
            escaped = true;
        } else if ch == quote {
            break;
        }
    }
    i
}

fn take_word(cells: &[CellChar], start: usize) -> usize {
    let mut i = start;
    while i < cells.len() {
        let ch = cells[i].ch;
        if ch == ' ' || matches!(ch, '|' | '&' | ';' | '>' | '<' | '(' | ')') {
            break;
        }
        i += 1;
    }
    i.max(start + 1)
}

fn push_span(out: &mut Vec<(Match, u32)>, slice: &[CellChar], color: u32) {
    let (Some(first), Some(last)) = (slice.first(), slice.last()) else {
        return;
    };
    out.push((
        Match {
            start: Point {
                line: first.line,
                col: first.col,
            },
            end: Point {
                line: last.line,
                col: last.col,
            },
        },
        color,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(text: &str) -> Vec<CellChar> {
        text.chars()
            .enumerate()
            .map(|(col, ch)| CellChar { line: 0, col, ch })
            .collect()
    }

    fn typed(output: &str) -> Option<String> {
        let mut term = Terminal::new(dimaster_grid::Size { rows: 3, cols: 40 });
        term.advance(b"\x1b]133;A\x07> \x1b]133;B\x07");
        term.advance(output.as_bytes());
        input_text_at_end(&term)
    }

    #[test]
    fn open_completion_list_hides_the_suggestion() {
        assert_eq!(typed("cd d\x1b7\r\ndocs/  da/\x1b8"), None);
        assert_eq!(typed("cd d").as_deref(), Some("cd d"));
    }

    #[test]
    fn colored_input_is_read_whole() {
        // zsh-syntax-highlighting красит команду, RPROMPT стоит далеко справа
        assert_eq!(typed("\x1b[32mecho\x1b[0m he").as_deref(), Some("echo he"));
        let rprompt = "\x1b[32mls\x1b[0m\x1b7\x1b[30G\x1b[34mmain\x1b[0m\x1b8";
        assert_eq!(typed(rprompt).as_deref(), Some("ls"));
        // серая подсказка плагина сразу за курсором: свою не рисуем
        assert_eq!(typed("ec\x1b7\x1b[90mho hi\x1b[0m\x1b8"), None);
    }

    #[test]
    fn highlights_valid_invalid_flags_and_operators() {
        let mut cache = CommandCache::default();
        let palette = [0; 16].map(|_| 0x111111);
        let mut pal = palette;
        pal[1] = 0xff0000;
        pal[2] = 0x00ff00;
        pal[3] = 0xffff00;
        pal[4] = 0x0000ff;
        pal[5] = 0xff00ff;
        let spans = tokenize(&cells("cd -L && nope_cmd_xyz 'hi'"), &pal, None, &mut cache);
        assert_eq!(spans[0].1, pal[2]); // cd (builtin) -> green
        assert_eq!(spans[1].1, pal[4]); // -L -> flag blue
        assert_eq!(spans[2].1, pal[5]); // && -> operator
        assert_eq!(spans[3].1, pal[1]); // nope_cmd_xyz -> red
        assert_eq!(spans[4].1, pal[3]); // 'hi' -> yellow string
    }

    #[test]
    fn osc_133b_marks_editable_range_after_prompt() {
        let mut term = Terminal::new(dimaster_grid::Size { rows: 5, cols: 40 });
        term.advance(b"\x1b]133;A\x07prompt> \x1b]133;B\x07echo 'ok'");
        let mut cache = CommandCache::default();
        let mut pal = [0u32; 16];
        pal[2] = 0xa6e3a1;
        pal[3] = 0xf9e2af;
        let spans = highlight_input(&term, &pal, &mut cache);
        assert_eq!(spans.len(), 2);
        assert!(spans[0].0.contains(0, 8));
        assert!(!spans[0].0.contains(0, 7));
        assert_eq!(spans[0].1, pal[2]);
        assert_eq!(spans[1].1, pal[3]);
    }
}
