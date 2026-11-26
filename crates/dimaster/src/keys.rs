//! Кодирование клавиш в последовательности xterm.

use winit::keyboard::{Key, ModifiersState, NamedKey};

/// Режимы терминала, влияющие на кодирование.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct KeyModes {
    /// DECCKM: стрелки шлют `ESC O` вместо `ESC [`.
    pub(crate) app_cursor: bool,
    /// Протокол Kitty, флаг «disambiguate escape codes».
    pub(crate) disambiguate: bool,
}

/// Байты для PTY; `None` — клавиша ничего не отправляет.
pub(crate) fn encode(
    key: &Key,
    text: Option<&str>,
    mods: ModifiersState,
    modes: KeyModes,
) -> Option<Vec<u8>> {
    if modes.disambiguate
        && let Some(bytes) = unambiguous(key, mods)
    {
        return Some(bytes);
    }
    if let Key::Named(named) = key
        && let Some(bytes) = named_key(*named, mods, modes.app_cursor)
    {
        return Some(bytes);
    }
    if mods.control_key()
        && let Key::Character(chars) = key
        && let Some(byte) = control_byte(chars.chars().next()?)
    {
        return Some(with_alt(mods, vec![byte]));
    }
    let text = text.filter(|t| !t.is_empty())?;
    Some(with_alt(mods, text.as_bytes().to_vec()))
}

// `CSI код;модификаторы u` для клавиш, неотличимых в старой кодировке
fn unambiguous(key: &Key, mods: ModifiersState) -> Option<Vec<u8>> {
    let chord = mods.control_key() || mods.alt_key() || mods.super_key();
    let modified = chord || mods.shift_key();
    let code = match key {
        Key::Named(NamedKey::Escape) => 27,
        Key::Named(NamedKey::Enter) if modified => 13,
        Key::Named(NamedKey::Tab) if modified => 9,
        Key::Named(NamedKey::Backspace) if modified => 127,
        Key::Named(NamedKey::Space) if chord => 32,
        Key::Character(text) if chord => u32::from(text.chars().next()?.to_ascii_lowercase()),
        // CSI R занят отчётом о позиции курсора
        Key::Named(NamedKey::F3) => return Some(tilde(13, modifier_code(mods))),
        _ => return None,
    };
    let sequence = match modifier_code(mods) {
        1 => format!("\x1b[{code}u"),
        modifiers => format!("\x1b[{code};{modifiers}u"),
    };
    Some(sequence.into_bytes())
}

fn tilde(number: u8, code: u8) -> Vec<u8> {
    match code {
        1 => format!("\x1b[{number}~"),
        _ => format!("\x1b[{number};{code}~"),
    }
    .into_bytes()
}

fn with_alt(mods: ModifiersState, mut bytes: Vec<u8>) -> Vec<u8> {
    if mods.alt_key() {
        bytes.insert(0, 0x1b);
    }
    bytes
}

fn control_byte(ch: char) -> Option<u8> {
    match ch.to_ascii_lowercase() {
        c @ 'a'..='z' => Some(c as u8 - b'a' + 1),
        ' ' | '@' | '2' => Some(0),
        '[' | '3' => Some(0x1b),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '/' | '7' => Some(0x1f),
        '8' | '?' => Some(0x7f),
        _ => None,
    }
}

fn named_key(key: NamedKey, mods: ModifiersState, app_cursor: bool) -> Option<Vec<u8>> {
    let simple: &[u8] = match key {
        NamedKey::Enter => b"\r",
        NamedKey::Escape => b"\x1b",
        NamedKey::Backspace if mods.control_key() => b"\x08",
        NamedKey::Backspace => b"\x7f",
        NamedKey::Tab if mods.shift_key() => return Some(b"\x1b[Z".to_vec()),
        NamedKey::Tab => b"\t",
        NamedKey::Space if mods.control_key() => b"\0",
        NamedKey::Space => b" ",
        _ => return function_key(key, mods, app_cursor),
    };
    Some(with_alt(mods, simple.to_vec()))
}

// xterm: 1 + Shift(1) + Alt(2) + Ctrl(4) + Super(8)
fn modifier_code(mods: ModifiersState) -> u8 {
    1 + u8::from(mods.shift_key())
        + 2 * u8::from(mods.alt_key())
        + 4 * u8::from(mods.control_key())
        + 8 * u8::from(mods.super_key())
}

fn function_key(key: NamedKey, mods: ModifiersState, app_cursor: bool) -> Option<Vec<u8>> {
    let code = modifier_code(mods);
    if let Some(letter) = letter_key(key) {
        // F1–F4 всегда идут через SS3, стрелки — только в app cursor mode
        let ss3 = app_cursor || matches!(letter, 'P'..='S');
        let sequence = match (code, ss3) {
            (1, true) => format!("\x1bO{letter}"),
            (1, false) => format!("\x1b[{letter}"),
            _ => format!("\x1b[1;{code}{letter}"),
        };
        return Some(sequence.into_bytes());
    }
    Some(tilde(tilde_key(key)?, code))
}

fn letter_key(key: NamedKey) -> Option<char> {
    Some(match key {
        NamedKey::ArrowUp => 'A',
        NamedKey::ArrowDown => 'B',
        NamedKey::ArrowRight => 'C',
        NamedKey::ArrowLeft => 'D',
        NamedKey::Home => 'H',
        NamedKey::End => 'F',
        NamedKey::F1 => 'P',
        NamedKey::F2 => 'Q',
        NamedKey::F3 => 'R',
        NamedKey::F4 => 'S',
        _ => return None,
    })
}

fn tilde_key(key: NamedKey) -> Option<u8> {
    Some(match key {
        NamedKey::Insert => 2,
        NamedKey::Delete => 3,
        NamedKey::PageUp => 5,
        NamedKey::PageDown => 6,
        NamedKey::F5 => 15,
        NamedKey::F6 => 17,
        NamedKey::F7 => 18,
        NamedKey::F8 => 19,
        NamedKey::F9 => 20,
        NamedKey::F10 => 21,
        NamedKey::F11 => 23,
        NamedKey::F12 => 24,
        _ => return None,
    })
}

/// Подготавливает вставку из буфера обмена.
///
/// ESC вырезается, чтобы вставленный текст не мог закрыть bracketed paste
/// и выполнить команду без ведома пользователя.
pub(crate) fn paste(text: &str, bracketed: bool) -> Vec<u8> {
    let clean = text
        .replace('\x1b', "")
        .replace("\r\n", "\r")
        .replace('\n', "\r");
    if bracketed {
        format!("\x1b[200~{clean}\x1b[201~").into_bytes()
    } else {
        clean.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modes(app_cursor: bool, disambiguate: bool) -> KeyModes {
        KeyModes {
            app_cursor,
            disambiguate,
        }
    }

    #[test]
    fn kitty_disambiguation() {
        let kitty = modes(false, true);
        let escape = Key::Named(NamedKey::Escape);
        let none = ModifiersState::empty();
        assert_eq!(encode(&escape, None, none, kitty).unwrap(), b"\x1b[27u");
        let i = Key::Character("i".into());
        assert_eq!(
            encode(&i, Some("i"), ModifiersState::CONTROL, kitty).unwrap(),
            b"\x1b[105;5u"
        );
        assert_eq!(encode(&i, Some("i"), none, kitty).unwrap(), b"i");
        let enter = Key::Named(NamedKey::Enter);
        assert_eq!(encode(&enter, None, none, kitty).unwrap(), b"\r");
    }

    fn named(key: NamedKey, mods: ModifiersState, app: bool) -> Vec<u8> {
        encode(&Key::Named(key), None, mods, modes(app, false)).unwrap()
    }

    #[test]
    fn arrows_follow_cursor_mode_and_modifiers() {
        let none = ModifiersState::empty();
        assert_eq!(named(NamedKey::ArrowUp, none, false), b"\x1b[A");
        assert_eq!(named(NamedKey::ArrowUp, none, true), b"\x1bOA");
        assert_eq!(
            named(NamedKey::ArrowLeft, ModifiersState::CONTROL, true),
            b"\x1b[1;5D"
        );
        assert_eq!(
            named(NamedKey::Delete, ModifiersState::SHIFT, false),
            b"\x1b[3;2~"
        );
    }

    #[test]
    fn control_alt_and_plain_text() {
        let c = Key::Character("c".into());
        assert_eq!(
            encode(&c, Some("c"), ModifiersState::CONTROL, modes(false, false)).unwrap(),
            [3]
        );
        assert_eq!(
            encode(&c, Some("c"), ModifiersState::ALT, modes(false, false)).unwrap(),
            b"\x1bc"
        );
        let ru = Key::Character("ё".into());
        assert_eq!(
            encode(&ru, Some("ё"), ModifiersState::empty(), modes(false, false)).unwrap(),
            "ё".as_bytes()
        );
    }

    #[test]
    fn paste_cannot_escape_brackets() {
        assert_eq!(paste("a\x1b[201~\nb", true), b"\x1b[200~a[201~\rb\x1b[201~");
    }
}
