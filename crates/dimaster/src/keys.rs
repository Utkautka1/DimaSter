//! Кодирование клавиш (xterm + полный протокол Kitty) и безопасная вставка.

use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

pub(crate) const KITTY_DISAMBIGUATE: u16 = 1;
pub(crate) const KITTY_EVENTS: u16 = 2;
pub(crate) const KITTY_ALTERNATE: u16 = 4;
pub(crate) const KITTY_ALL_KEYS: u16 = 8;
pub(crate) const KITTY_TEXT: u16 = 16;

/// Тип события клавиатуры.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyAction {
    #[default]
    Press = 1,
    Repeat = 2,
    Release = 3,
}

/// Режимы терминала, влияющие на кодирование.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct KeyModes {
    /// DECCKM: стрелки шлют `ESC O` вместо `ESC [`.
    pub(crate) app_cursor: bool,
    /// Флаги клавиатурного протокола Kitty (биты 1..=16).
    pub(crate) kitty: u16,
}

/// Данные нажатой или отпущенной клавиши для кодирования.
pub(crate) struct KeyInput<'a> {
    pub(crate) key: &'a Key,
    pub(crate) logical: &'a Key,
    pub(crate) physical: PhysicalKey,
    pub(crate) text: Option<&'a str>,
    pub(crate) mods: ModifiersState,
    pub(crate) action: KeyAction,
}

/// Байты для PTY; `None` — клавиша ничего не отправляет.
pub(crate) fn encode(input: &KeyInput<'_>, modes: KeyModes) -> Option<Vec<u8>> {
    if input.action == KeyAction::Release && (modes.kitty & KITTY_EVENTS == 0) {
        return None;
    }
    if modes.kitty != 0
        && let Some(bytes) = kitty_encode(input, modes)
    {
        return Some(bytes);
    }
    if input.action == KeyAction::Release {
        return None;
    }
    legacy_encode(
        input.key,
        input.physical,
        input.text,
        input.mods,
        modes.app_cursor,
    )
}

fn legacy_encode(
    key: &Key,
    physical: PhysicalKey,
    text: Option<&str>,
    mods: ModifiersState,
    app_cursor: bool,
) -> Option<Vec<u8>> {
    if let Key::Named(named) = key
        && let Some(bytes) = named_key(*named, mods, app_cursor)
    {
        return Some(bytes);
    }
    // На Linux/XKB нажатие Numpad 5 при выключенном NumLock даёт KP_Begin (Unidentified).
    if matches!(key, Key::Unidentified(_)) && physical == PhysicalKey::Code(KeyCode::Numpad5) {
        return function_key(NamedKey::Clear, mods, app_cursor);
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

fn kitty_encode(input: &KeyInput<'_>, modes: KeyModes) -> Option<Vec<u8>> {
    let flags = modes.kitty;
    let all = flags & KITTY_ALL_KEYS != 0;
    let events = flags & KITTY_EVENTS != 0;
    let event_num = if events { input.action as u8 } else { 1 };
    if let Some(bytes) = kitty_special(input, flags, event_num) {
        return Some(bytes);
    }
    if let Some(bytes) = kitty_char(input, flags, event_num, all) {
        return Some(bytes);
    }
    // при flag 2 без flag 8 обычные клавиши при отпускании молчат
    if input.action == KeyAction::Release {
        return None;
    }
    None
}

fn kitty_special(input: &KeyInput<'_>, flags: u16, event: u8) -> Option<Vec<u8>> {
    let all = flags & KITTY_ALL_KEYS != 0;
    let mod_code = modifier_code(input.mods);
    if let Key::Named(named) = input.key {
        if let Some(letter) = kitty_letter(*named, flags) {
            return Some(format_letter(letter, mod_code, event));
        }
        if let Some(num) = kitty_tilde(*named, flags) {
            return Some(format_tilde(num, mod_code, event));
        }
        if let Some(code) = kitty_csi_u_named(*named, input.physical, input.mods, all) {
            return Some(format_csi_u(code, None, None, mod_code, event, None));
        }
    }
    if let Some(bytes) = kitty_numpad_begin(input, flags, mod_code, event) {
        return Some(bytes);
    }
    modifier_key_code(input.physical, all)
        .map(|code| format_csi_u(code, None, None, mod_code, event, None))
}

fn kitty_numpad_begin(
    input: &KeyInput<'_>,
    flags: u16,
    mod_code: u8,
    event: u8,
) -> Option<Vec<u8>> {
    if !matches!(input.key, Key::Unidentified(_))
        || input.physical != PhysicalKey::Code(KeyCode::Numpad5)
    {
        return None;
    }
    let all = flags & KITTY_ALL_KEYS != 0;
    Some(
        if all || flags & KITTY_DISAMBIGUATE != 0 || mod_code > 1 || event > 1 {
            format_csi_u(57427, None, None, mod_code, event, None)
        } else {
            format_letter('E', mod_code, event)
        },
    )
}

fn kitty_char(input: &KeyInput<'_>, flags: u16, event: u8, all: bool) -> Option<Vec<u8>> {
    let Key::Character(chars) = input.key else {
        return None;
    };
    let ch = chars.chars().next()?;
    let chord = input.mods.control_key() || input.mods.alt_key() || input.mods.super_key();
    let use_csi = all || (chord && flags & KITTY_DISAMBIGUATE != 0);
    if !use_csi {
        return None;
    }
    let kp = kitty_numpad_char(input.physical);
    let code = kp.unwrap_or_else(|| u32::from(ch.to_ascii_lowercase()));
    let (shifted, base) = if kp.is_some() {
        (None, None)
    } else {
        alternate_codes(input, ch, code, flags & KITTY_ALTERNATE != 0)
    };
    let text = associated_text(input, flags & KITTY_TEXT != 0);
    Some(format_csi_u(
        code,
        shifted,
        base,
        modifier_code(input.mods),
        event,
        text.as_deref(),
    ))
}

fn alternate_codes(
    input: &KeyInput<'_>,
    ch: char,
    code: u32,
    enabled: bool,
) -> (Option<u32>, Option<u32>) {
    if !enabled {
        return (None, None);
    }
    let shifted = if input.mods.shift_key()
        && let Key::Character(logical) = input.logical
        && let Some(s) = logical.chars().next()
        && u32::from(s) != code
    {
        Some(u32::from(s))
    } else {
        None
    };
    let base = us_base_char(input.physical)
        .map(u32::from)
        .filter(|&b| b != code && !ch.is_ascii_alphanumeric());
    (shifted, base)
}

fn associated_text(input: &KeyInput<'_>, enabled: bool) -> Option<String> {
    if !enabled || input.action == KeyAction::Release {
        return None;
    }
    let text = input.text.filter(|t| !t.is_empty())?;
    let codes: Vec<String> = text
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| u32::from(c).to_string())
        .collect();
    (!codes.is_empty()).then(|| codes.join(":"))
}

fn format_csi_u(
    code: u32,
    shifted: Option<u32>,
    base: Option<u32>,
    mods: u8,
    event: u8,
    text: Option<&str>,
) -> Vec<u8> {
    let mut out = format!("\x1b[{code}");
    match (shifted, base) {
        (Some(s), Some(b)) => out.push_str(&format!(":{s}:{b}")),
        (Some(s), None) => out.push_str(&format!(":{s}")),
        (None, Some(b)) => out.push_str(&format!("::{b}")),
        (None, None) => {}
    }
    append_mods_and_text(&mut out, mods, event, text);
    out.push('u');
    out.into_bytes()
}

fn append_mods_and_text(out: &mut String, mods: u8, event: u8, text: Option<&str>) {
    if event > 1 {
        out.push_str(&format!(";{mods}:{event}"));
    } else if mods > 1 {
        out.push_str(&format!(";{mods}"));
    } else if text.is_some() {
        out.push_str(";1");
    }
    if let Some(text) = text {
        out.push(';');
        out.push_str(text);
    }
}

fn format_letter(letter: char, mods: u8, event: u8) -> Vec<u8> {
    if event > 1 {
        format!("\x1b[1;{mods}:{event}{letter}").into_bytes()
    } else if mods > 1 {
        format!("\x1b[1;{mods}{letter}").into_bytes()
    } else if matches!(letter, 'P'..='S') {
        format!("\x1bO{letter}").into_bytes()
    } else {
        format!("\x1b[{letter}").into_bytes()
    }
}

fn format_tilde(number: u8, mods: u8, event: u8) -> Vec<u8> {
    if event > 1 {
        format!("\x1b[{number};{mods}:{event}~").into_bytes()
    } else if mods > 1 {
        format!("\x1b[{number};{mods}~").into_bytes()
    } else {
        format!("\x1b[{number}~").into_bytes()
    }
}

fn kitty_letter(key: NamedKey, flags: u16) -> Option<char> {
    // F3 в Kitty использует CSI 13 ~ вместо CSI R, чтобы не путаться с CPR
    if key == NamedKey::F3 && flags & (KITTY_DISAMBIGUATE | KITTY_ALL_KEYS) != 0 {
        return None;
    }
    letter_key(key)
}

fn kitty_tilde(key: NamedKey, flags: u16) -> Option<u8> {
    if key == NamedKey::F3 && flags & (KITTY_DISAMBIGUATE | KITTY_ALL_KEYS) != 0 {
        return Some(13);
    }
    tilde_key(key)
}

fn kitty_csi_u_named(
    key: NamedKey,
    physical: PhysicalKey,
    mods: ModifiersState,
    all: bool,
) -> Option<u32> {
    let modified = mods.shift_key() || mods.control_key() || mods.alt_key() || mods.super_key();
    let chord = mods.control_key() || mods.alt_key() || mods.super_key();
    Some(match key {
        NamedKey::Escape => 27,
        NamedKey::Enter
            if (all || modified) && physical == PhysicalKey::Code(KeyCode::NumpadEnter) =>
        {
            57414
        }
        NamedKey::Enter if all || modified => 13,
        NamedKey::Tab if all || modified => 9,
        NamedKey::Backspace if all || modified => 127,
        NamedKey::Space if all || chord => 32,
        NamedKey::CapsLock if all => 57358,
        NamedKey::ScrollLock if all => 57359,
        NamedKey::NumLock if all => 57360,
        NamedKey::PrintScreen if all => 57361,
        NamedKey::Pause if all => 57362,
        NamedKey::ContextMenu if all => 57363,
        _ => return None,
    })
}

fn kitty_numpad_char(physical: PhysicalKey) -> Option<u32> {
    use KeyCode::*;
    let PhysicalKey::Code(code) = physical else {
        return None;
    };
    Some(match code {
        Numpad0 => 57399,
        Numpad1 => 57400,
        Numpad2 => 57401,
        Numpad3 => 57402,
        Numpad4 => 57403,
        Numpad5 => 57404,
        Numpad6 => 57405,
        Numpad7 => 57406,
        Numpad8 => 57407,
        Numpad9 => 57408,
        NumpadDecimal => 57409,
        NumpadDivide => 57410,
        NumpadMultiply => 57411,
        NumpadSubtract => 57412,
        NumpadAdd => 57413,
        NumpadEqual => 57415,
        NumpadComma => 57416,
        _ => return None,
    })
}

fn modifier_key_code(physical: PhysicalKey, all: bool) -> Option<u32> {
    if !all {
        return None;
    }
    let PhysicalKey::Code(code) = physical else {
        return None;
    };
    Some(match code {
        KeyCode::ShiftLeft => 57441,
        KeyCode::ControlLeft => 57442,
        KeyCode::AltLeft => 57443,
        KeyCode::SuperLeft => 57444,
        KeyCode::ShiftRight => 57447,
        KeyCode::ControlRight => 57448,
        KeyCode::AltRight => 57449,
        KeyCode::SuperRight => 57450,
        _ => return None,
    })
}

fn us_base_char(physical: PhysicalKey) -> Option<char> {
    let PhysicalKey::Code(code) = physical else {
        return None;
    };
    us_letter_or_digit(code).or_else(|| us_punctuation(code))
}

fn us_letter_or_digit(code: KeyCode) -> Option<char> {
    use KeyCode::*;
    let letters = [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    ];
    if let Some(pos) = letters.iter().position(|&c| c == code) {
        return Some((b'a' + pos as u8) as char);
    }
    let digits = [
        Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    ];
    digits
        .iter()
        .position(|&c| c == code)
        .map(|pos| (b'0' + pos as u8) as char)
}

fn us_punctuation(code: KeyCode) -> Option<char> {
    Some(match code {
        KeyCode::Minus => '-',
        KeyCode::Equal => '=',
        KeyCode::BracketLeft => '[',
        KeyCode::BracketRight => ']',
        KeyCode::Backslash => '\\',
        KeyCode::Semicolon => ';',
        KeyCode::Quote => '\'',
        KeyCode::Comma => ',',
        KeyCode::Period => '.',
        KeyCode::Slash => '/',
        KeyCode::Backquote => '`',
        _ => return None,
    })
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
        let ss3 = app_cursor || matches!(letter, 'P'..='S');
        let sequence = match (code, ss3) {
            (1, true) => format!("\x1bO{letter}"),
            (1, false) => format!("\x1b[{letter}"),
            _ => format!("\x1b[1;{code}{letter}"),
        };
        return Some(sequence.into_bytes());
    }
    Some(format_tilde(tilde_key(key)?, code, 1))
}

fn letter_key(key: NamedKey) -> Option<char> {
    Some(match key {
        NamedKey::ArrowUp => 'A',
        NamedKey::ArrowDown => 'B',
        NamedKey::ArrowRight => 'C',
        NamedKey::ArrowLeft => 'D',
        NamedKey::Clear => 'E',
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

/// Причина предупреждения при вставке.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PasteDanger {
    pub(crate) lines: usize,
    pub(crate) controls: bool,
}

/// Проверяет текст на многострочность без bracketed paste и опасные управляющие символы.
pub(crate) fn paste_danger(text: &str, bracketed: bool) -> Option<PasteDanger> {
    let lines = text.lines().count().max(1);
    let has_newline = text.contains('\n') || text.contains('\r');
    let controls = text.chars().any(is_dangerous_control);
    if controls || (!bracketed && has_newline) {
        Some(PasteDanger { lines, controls })
    } else {
        None
    }
}

fn is_dangerous_control(ch: char) -> bool {
    matches!(ch, '\x00'..='\x08' | '\x0b'..='\x0c' | '\x0e'..='\x1f' | '\x7f' | '\u{0080}'..='\u{009f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Подготавливает вставку: вырезает ESC и опасные управляющие байты.
pub(crate) fn paste(text: &str, bracketed: bool) -> Vec<u8> {
    let clean: String = text
        .chars()
        .filter(|&c| !is_dangerous_control(c))
        .collect::<String>()
        .replace("\r\n", "\r")
        .replace('\n', "\r");
    if bracketed {
        format!("\x1b[200~{clean}\x1b[201~").into_bytes()
    } else {
        clean.into_bytes()
    }
}

/// Вставка в одну строку: заменяет переводы строк пробелами.
pub(crate) fn paste_single_line(text: &str, bracketed: bool) -> Vec<u8> {
    let flat = text.replace("\r\n", " ").replace(['\r', '\n'], " ");
    paste(&flat, bracketed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input<'a>(key: &'a Key, mods: ModifiersState) -> KeyInput<'a> {
        KeyInput {
            key,
            logical: key,
            physical: PhysicalKey::Unidentified(winit::keyboard::NativeKeyCode::Unidentified),
            text: None,
            mods,
            action: KeyAction::Press,
        }
    }

    fn modes(app_cursor: bool, kitty: u16) -> KeyModes {
        KeyModes { app_cursor, kitty }
    }

    #[test]
    fn kitty_disambiguation_and_events() {
        let kitty = modes(false, KITTY_DISAMBIGUATE | KITTY_EVENTS);
        let escape = Key::Named(NamedKey::Escape);
        let none = ModifiersState::empty();
        assert_eq!(encode(&input(&escape, none), kitty).unwrap(), b"\x1b[27u");
        let mut rel = input(&escape, none);
        rel.action = KeyAction::Release;
        assert_eq!(encode(&rel, kitty).unwrap(), b"\x1b[27;1:3u");
        let i = Key::Character("i".into());
        let mut ci = input(&i, ModifiersState::CONTROL);
        ci.text = Some("i");
        assert_eq!(encode(&ci, kitty).unwrap(), b"\x1b[105;5u");
    }

    #[test]
    fn kitty_all_keys_alternate_and_text() {
        let flags =
            KITTY_DISAMBIGUATE | KITTY_EVENTS | KITTY_ALTERNATE | KITTY_ALL_KEYS | KITTY_TEXT;
        let a = Key::Character("a".into());
        let shift_a = Key::Character("A".into());
        let mut ki = KeyInput {
            key: &a,
            logical: &shift_a,
            physical: PhysicalKey::Code(KeyCode::KeyA),
            text: Some("A"),
            mods: ModifiersState::SHIFT,
            action: KeyAction::Press,
        };
        assert_eq!(
            encode(&ki, modes(false, flags)).unwrap(),
            b"\x1b[97:65;2;65u"
        );
        ki.action = KeyAction::Repeat;
        assert_eq!(
            encode(&ki, modes(false, flags)).unwrap(),
            b"\x1b[97:65;2:2;65u"
        );
    }

    #[test]
    fn kitty_base_layout_for_non_latin() {
        let flags = KITTY_ALTERNATE | KITTY_ALL_KEYS;
        let ru = Key::Character("ф".into());
        let ki = KeyInput {
            key: &ru,
            logical: &ru,
            physical: PhysicalKey::Code(KeyCode::KeyA),
            text: Some("ф"),
            mods: ModifiersState::empty(),
            action: KeyAction::Press,
        };
        assert_eq!(encode(&ki, modes(false, flags)).unwrap(), b"\x1b[1092::97u");
    }

    #[test]
    fn numpad_digits_and_enter() {
        let none = ModifiersState::empty();
        let one = Key::Character("1".into());
        let mut kp1 = input(&one, none);
        kp1.physical = PhysicalKey::Code(KeyCode::Numpad1);
        kp1.text = Some("1");
        assert_eq!(encode(&kp1, modes(true, 0)).unwrap(), b"1");
        assert_eq!(
            encode(&kp1, modes(false, KITTY_DISAMBIGUATE)).unwrap(),
            b"1"
        );
        assert_eq!(
            encode(&kp1, modes(false, KITTY_ALL_KEYS)).unwrap(),
            b"\x1b[57400u"
        );

        let enter = Key::Named(NamedKey::Enter);
        let mut kp_enter = input(&enter, none);
        kp_enter.physical = PhysicalKey::Code(KeyCode::NumpadEnter);
        assert_eq!(encode(&kp_enter, modes(true, 0)).unwrap(), b"\r");
        assert_eq!(
            encode(&kp_enter, modes(false, KITTY_ALL_KEYS)).unwrap(),
            b"\x1b[57414u"
        );
    }

    #[test]
    fn numpad_begin_and_nav() {
        let none = ModifiersState::empty();
        let unid = Key::Unidentified(winit::keyboard::NativeKey::Unidentified);
        let mut kp5 = input(&unid, none);
        kp5.physical = PhysicalKey::Code(KeyCode::Numpad5);
        assert_eq!(encode(&kp5, modes(false, 0)).unwrap(), b"\x1b[E");
        assert_eq!(encode(&kp5, modes(true, 0)).unwrap(), b"\x1bOE");
        assert_eq!(
            encode(&kp5, modes(false, KITTY_DISAMBIGUATE)).unwrap(),
            b"\x1b[57427u"
        );
    }

    #[test]
    fn paste_safety_and_single_line() {
        assert_eq!(
            paste("a\x1b[201~\x03\nb", true),
            b"\x1b[200~a[201~\rb\x1b[201~"
        );
        assert!(paste_danger("echo 1\necho 2", false).is_some());
        assert!(paste_danger("echo 1\necho 2", true).is_none());
        assert!(paste_danger("hello\x1b[201~", true).is_some());
        assert_eq!(paste_single_line("a\nb", false), b"a b");
    }
}
