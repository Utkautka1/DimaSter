//! Виды и цвета подчёркивания, гиперссылки OSC 8.

use dimaster_grid::{Color, Point, Size, Terminal, Underline};

fn run(input: &str) -> Terminal {
    let mut term = Terminal::new(Size { rows: 4, cols: 30 });
    term.advance(input.as_bytes());
    term
}

fn style(term: &Terminal, col: usize) -> dimaster_grid::Style {
    term.screen().rows()[0].cells[col].style
}

#[test]
fn underline_kinds_and_colors() {
    let term =
        run("\x1b[4:3;58:2::255:0:0mA\x1b[21mB\x1b[4:5mC\x1b[24mD\x1b[4;58;5;9mE\x1b[59mF\x1b[0mG");
    let kinds: Vec<_> = (0..7).map(|c| style(&term, c).underline).collect();
    use Underline::{Curly, Dashed, Double, None as Off, Single};
    assert_eq!(kinds, [Curly, Double, Dashed, Off, Single, Single, Off]);
    assert_eq!(style(&term, 0).underline_color, Color::Rgb(255, 0, 0));
    assert_eq!(style(&term, 4).underline_color, Color::Indexed(9));
    assert_eq!(style(&term, 5).underline_color, Color::Default);
}

#[test]
fn hyperlinks_cover_their_text_and_survive_sgr() {
    let term = run("a \x1b]8;;https://example.com/x\x07cli\x1b[0mck\x1b]8;;\x07 b");
    let at = |col| term.link_at(Point { line: 0, col });
    let (range, uri) = at(4).unwrap();
    assert_eq!(uri, "https://example.com/x");
    assert_eq!((range.start.col, range.end.col), (2, 6));
    assert!(at(0).is_none() && at(8).is_none());
    let all = term.links_in(0..4);
    assert_eq!((all.len(), all[0].1.as_str()), (1, "https://example.com/x"));
}

#[test]
fn erasing_removes_the_link() {
    let term = run("\x1b]8;;https://e.com\x07xx\r\x1b[K");
    assert!(term.link_at(Point { line: 0, col: 0 }).is_none());
}
