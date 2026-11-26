//! Поведение экрана на типичных последовательностях shell и TUI-программ.

use dimaster_grid::{Color, Flags, Size, Terminal};
use proptest::prelude::*;

fn run(rows: usize, cols: usize, input: &str) -> Terminal {
    let mut term = Terminal::new(Size { rows, cols });
    term.advance(input.as_bytes());
    term
}

#[test]
fn wraps_lazily_at_right_edge() {
    let term = run(3, 4, "abcd");
    assert_eq!((term.cursor().row, term.cursor().col), (0, 3));
    assert_eq!(run(3, 4, "abcde").text(), "abcd\ne");
}

#[test]
fn scrolled_lines_go_to_history() {
    let term = run(2, 10, "1\r\n2\r\n3");
    assert_eq!(term.text(), "2\n3");
    assert_eq!(term.screen().history().count(), 1);
}

#[test]
fn alternate_screen_restores_main() {
    let term = run(3, 10, "main\x1b[?1049h\x1b[Hvim\x1b[?1049l");
    assert_eq!(term.text(), "main");
}

#[test]
fn wide_and_combining_characters() {
    let term = run(1, 10, "漢e\u{301}");
    assert_eq!(term.text(), "漢e\u{301}");
    assert_eq!(term.cursor().col, 3);
    assert_eq!(run(1, 10, "漢\rx").text(), "x");
}

#[test]
fn sgr_colors_in_both_forms() {
    let term = run(1, 10, "\x1b[1;38;2;1;2;3;48:5:7mA");
    let style = term.screen().rows()[0].cells[0].style;
    assert_eq!(
        (style.fg, style.bg),
        (Color::Rgb(1, 2, 3), Color::Indexed(7))
    );
    assert!(style.flags.contains(Flags::BOLD));
}

#[test]
fn scroll_region_and_line_editing() {
    assert_eq!(
        run(4, 5, "a\r\nb\r\nc\r\nd\x1b[2;3r\x1b[3H\n").text(),
        "a\nc\n\nd"
    );
    assert_eq!(run(3, 5, "a\r\nb\x1b[H\x1b[M").text(), "b");
    assert_eq!(run(1, 6, "abcd\r\x1b[2P").text(), "cd");
}

#[test]
fn replies_to_queries() {
    let mut term = run(5, 5, "ab\x1b[6n");
    assert_eq!(term.take_reply(), b"\x1b[1;3R");
}

#[test]
fn title_cwd_and_line_drawing() {
    let term = run(1, 10, "\x1b]0;t\x07\x1b]7;file://h/tmp/a%20b\x07\x1b(0qx");
    assert_eq!((term.title(), term.cwd()), ("t", Some("/tmp/a b")));
    assert_eq!(term.text(), "─│");
}

#[test]
fn shrinking_keeps_cursor_line() {
    let mut term = run(4, 10, "1\r\n2\r\n3\r\n4");
    term.resize(Size { rows: 2, cols: 10 });
    assert_eq!(term.text(), "3\n4");
}

#[test]
fn shrinking_under_alternate_screen_keeps_main() {
    let mut term = run(4, 10, "1\r\n2\r\n3\r\n4\x1b[?1049h");
    term.resize(Size { rows: 2, cols: 10 });
    term.advance(b"\x1b[?1049l");
    assert_eq!(term.text(), "3\n4");
    assert_eq!(term.cursor().row, 1);
}

// Смесь осмысленных фрагментов чаще находит сбои, чем чистый шум
fn fragment() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        "\x1b\\[[0-9;]{0,8}[@-~]".prop_map(String::into_bytes),
        "\x1b\\[\\?(6|7|47|1049)[hl]".prop_map(String::into_bytes),
        "\x1b[78DEMHc]".prop_map(String::into_bytes),
        "(a|漢|\u{301}|\r|\n|\t){1,8}".prop_map(String::into_bytes),
        proptest::collection::vec(any::<u8>(), 0..8),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn survives_garbage_and_resizes(
        parts in proptest::collection::vec((fragment(), 0usize..8, 0usize..12), 0..64),
    ) {
        let mut term = Terminal::new(Size { rows: 4, cols: 6 });
        for (bytes, rows, cols) in parts {
            term.advance(&bytes);
            if rows < 5 {
                term.resize(Size { rows, cols });
            }
            let cursor = term.cursor();
            prop_assert!(cursor.row < term.screen().rows().len());
            prop_assert!(cursor.col < term.screen().rows()[0].cells.len());
        }
    }
}
