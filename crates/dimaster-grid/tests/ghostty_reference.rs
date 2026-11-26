//! Последовательности из справочника Ghostty, которых раньше не было.

use dimaster_grid::{Policy, Size, Terminal};

fn term(input: &str) -> Terminal {
    let mut term = Terminal::new(Size { rows: 4, cols: 10 });
    term.advance(input.as_bytes());
    term
}

fn reply(input: &str) -> String {
    String::from_utf8(term(input).take_reply()).unwrap()
}

#[test]
fn text_wraps_and_scrolls_inside_side_margins() {
    // поля 3..6, строки 1..2; шесть букв переносятся внутри полей
    let t = term("0123456789\r\n0123456789\x1b[?69h\x1b[3;6s\x1b[1;3Habcdef");
    assert_eq!(t.text(), "01abcd6789\n01ef456789");
    let t = term("0123456789\r\n0123456789\x1b[?69h\x1b[3;6s\x1b[1;2r\x1b[1;1H\x1b[3G\x1b[S");
    assert_eq!(t.text(), "0123456789\n01    6789");
}

#[test]
fn editing_respects_margins_and_mode_off_restores_them() {
    let t = term("0123456789\x1b[?69h\x1b[3;6s\x1b[1;3H\x1b[2P");
    assert_eq!(t.text(), "0145  6789");
    let t = term("0123456789\x1b[?69h\x1b[3;6s\x1b[1;3H\x1b[1@\r\n");
    assert_eq!((t.text().as_str(), t.cursor().col), ("01 2346789", 2));
    let t = term("\x1b[?69h\x1b[3;6s\x1b[?69l0123456789");
    assert_eq!(t.text(), "0123456789");
}

#[test]
fn bell_shift_escape_pointer_and_progress() {
    let t = term("\x07\x07\x1b[>1s\x1b]22;crosshair\x1b\\\x1b]9;4;2;40\x07");
    assert_eq!(t.bells(), 2);
    assert!(t.modes().shift_escape);
    assert_eq!(t.pointer_shape(), Some("crosshair"));
    let progress = t.progress().unwrap();
    assert_eq!((progress.value, progress.failed), (40, true));
    assert!(term("\x1b]9;4;1;5\x07\x1b]9;4;0\x07").progress().is_none());
}

#[test]
fn colors_are_set_queried_and_reset() {
    assert!(reply("\x1b]17;#010203\x07\x1b]17;?\x07").contains("17;rgb:0101/0202/0303"));
    assert!(reply("\x1b]5;0;#ff0000\x07\x1b]5;0;?\x07").contains("5;0;rgb:ffff/0000/0000"));
    let t = term("\x1b]10;#111111;#222222\x07\x1b]111\x07\x1b]1;icon\x07");
    let colors = t.color_overrides();
    assert_eq!(
        (colors.foreground, colors.background),
        (Some(0x111111), None)
    );
}

#[test]
fn policy_forbids_changes() {
    let mut t = term("");
    let policy = Policy {
        title: false,
        colors: false,
        notifications: false,
        ..Policy::default()
    };
    t.set_policy(policy);
    t.advance(b"\x1b]2;x\x07\x1b]10;#111111\x07\x1b]9;hello\x07");
    assert_eq!((t.title(), t.color_overrides().foreground), ("", None));
    assert!(t.take_notifications().is_empty());
}
