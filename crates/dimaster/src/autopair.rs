//! Умное автозакрытие скобок и кавычек (аналог zsh-autopair) без плагинов shell.

use dimaster_grid::Terminal;
use winit::keyboard::{Key, ModifiersState, NamedKey};

const PAIRS: [(char, char); 5] = [('(', ')'), ('[', ']'), ('{', '}'), ('"', '"'), ('\'', '\'')];

/// Возвращает байты автопары/перешагивания/удаления или `None`, если ввод обычный.
pub(crate) fn handle(
    term: &Terminal,
    key: &Key,
    text: Option<&str>,
    mods: ModifiersState,
) -> Option<Vec<u8>> {
    if term.is_alternate() || !crate::syntax::editing_command(term) {
        return None;
    }
    if mods.control_key() || mods.alt_key() || mods.super_key() {
        return None;
    }
    let (left, under) = neighbors(term);
    if matches!(key, Key::Named(NamedKey::Backspace)) {
        return is_pair(left, under).then(|| b"\x1b[C\x7f\x7f".to_vec());
    }
    let ch = single_char(text)?;
    if closing_for(ch).is_some() && can_close_skip(ch, under) {
        return Some(b"\x1b[C".to_vec());
    }
    let close = PAIRS.iter().find(|(o, _)| *o == ch).map(|(_, c)| *c)?;
    can_open(ch, left, under).then(|| vec![ch as u8, close as u8, 0x1b, b'[', b'D'])
}

fn single_char(text: Option<&str>) -> Option<char> {
    let mut chars = text?.chars();
    let ch = chars.next()?;
    chars.next().is_none().then_some(ch)
}

fn neighbors(term: &Terminal) -> (Option<char>, Option<char>) {
    let cursor = term.cursor();
    let line = term.visible_line(0) + cursor.row as u64;
    let Some(row) = term.line(line) else {
        return (None, None);
    };
    let left = cursor
        .col
        .checked_sub(1)
        .and_then(|c| row.cells.get(c))
        .map(|c| c.ch);
    let under = row.cells.get(cursor.col).map(|c| c.ch);
    (left, under)
}

fn is_pair(left: Option<char>, under: Option<char>) -> bool {
    let (Some(l), Some(u)) = (left, under) else {
        return false;
    };
    PAIRS.contains(&(l, u))
}

fn closing_for(ch: char) -> Option<char> {
    PAIRS.iter().find(|(_, c)| *c == ch).map(|(o, _)| *o)
}

fn can_close_skip(ch: char, under: Option<char>) -> bool {
    under == Some(ch) && closing_for(ch).is_some()
}

fn can_open(ch: char, left: Option<char>, under: Option<char>) -> bool {
    let right_clear = under.is_none_or(|c| c == ' ' || matches!(c, ')' | ']' | '}' | ';' | '|'));
    if !right_clear {
        return false;
    }
    // Кавычку внутри слова (например don't) не удваиваем.
    if matches!(ch, '\'' | '"') {
        return !left.is_some_and(|c| c.is_alphanumeric() || c == '_');
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use dimaster_grid::Size;

    fn prompt_term(line: &str) -> Terminal {
        let mut term = Terminal::new(Size { rows: 4, cols: 30 });
        term.advance(b"\x1b]133;A\x07$ \x1b]133;B\x07");
        term.advance(line.as_bytes());
        term
    }

    #[test]
    fn opens_skips_and_deletes_pairs() {
        let none = ModifiersState::empty();
        let term = prompt_term("echo ");
        let open_q = Key::Character("\"".into());
        assert_eq!(
            handle(&term, &open_q, Some("\""), none).unwrap(),
            b"\"\"\x1b[D"
        );
        let inside = prompt_term("echo \"\"\x1b[D");
        assert_eq!(
            handle(&inside, &open_q, Some("\""), none).unwrap(),
            b"\x1b[C"
        );
        let bs = Key::Named(NamedKey::Backspace);
        assert_eq!(handle(&inside, &bs, None, none).unwrap(), b"\x1b[C\x7f\x7f");
        // Внутри слова одинарную кавычку не дублируем
        let word = prompt_term("echo don");
        assert!(handle(&word, &Key::Character("'".into()), Some("'"), none).is_none());
    }
}
