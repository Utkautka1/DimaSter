//! Переразбиение строк и выделение.

use dimaster_grid::{Point, Selection, SelectionKind, Size, Terminal};

fn run(rows: usize, cols: usize, input: &str) -> Terminal {
    let mut term = Terminal::new(Size { rows, cols });
    term.advance(input.as_bytes());
    term
}

fn resize(term: &mut Terminal, rows: usize, cols: usize) {
    term.resize(Size { rows, cols });
}

#[test]
fn wrapped_line_survives_narrowing_and_widening() {
    let mut term = run(4, 10, "abcdefgh\r\n> ");
    resize(&mut term, 4, 4);
    assert_eq!(term.text(), "abcd\nefgh\n>");
    assert_eq!((term.cursor().row, term.cursor().col), (2, 2));
    resize(&mut term, 4, 10);
    assert_eq!(term.text(), "abcdefgh\n>");
    assert_eq!((term.cursor().row, term.cursor().col), (1, 2));
}

#[test]
fn history_is_reflowed_and_pulled_back() {
    let mut term = run(2, 8, "123456\r\nab\r\ncd");
    resize(&mut term, 2, 3);
    resize(&mut term, 4, 8);
    assert_eq!(term.text(), "123456\nab\ncd");
    assert_eq!(term.screen().history().count(), 0);
}

#[test]
fn spaces_at_wrap_point_and_wide_chars_are_kept() {
    let mut term = run(3, 4, "ab  cd");
    resize(&mut term, 3, 10);
    assert_eq!(term.text(), "ab  cd");
    let mut wide = run(3, 4, "abc漢d");
    resize(&mut wide, 3, 10);
    assert_eq!(wide.text(), "abc漢d");
}

#[test]
fn pending_wrap_cursor_stays_after_last_char() {
    let mut term = run(3, 4, "abcd");
    resize(&mut term, 3, 8);
    term.advance(b"e");
    assert_eq!(term.text(), "abcde");
}

fn select(term: &Terminal, kind: SelectionKind, from: (u64, usize), to: (u64, usize)) -> String {
    let point = |(line, col)| Point { line, col };
    term.selected_text(Selection {
        kind,
        anchor: point(from),
        head: point(to),
    })
}

#[test]
fn selection_kinds() {
    let term = run(4, 12, "ls ~/a.txt x\r\nsecond");
    assert_eq!(
        select(&term, SelectionKind::Simple, (1, 2), (0, 11)),
        "x\nsec"
    );
    assert_eq!(
        select(&term, SelectionKind::Word, (0, 5), (0, 5)),
        "~/a.txt"
    );
    assert_eq!(select(&term, SelectionKind::Line, (1, 3), (1, 3)), "second");
    assert_eq!(
        select(&term, SelectionKind::Block, (1, 1), (0, 0)),
        "ls\nse"
    );
}

#[test]
fn soft_wrap_is_not_copied_as_newline() {
    let term = run(3, 4, "abcdef");
    assert_eq!(
        select(&term, SelectionKind::Simple, (0, 0), (1, 3)),
        "abcdef"
    );
}

#[test]
fn selection_follows_text_into_history() {
    let mut term = run(2, 8, "keep\r\n");
    let line = term.visible_line(0);
    term.advance(b"1\r\n2\r\n3");
    assert_eq!(
        select(&term, SelectionKind::Line, (line, 0), (line, 0)),
        "keep"
    );
}
