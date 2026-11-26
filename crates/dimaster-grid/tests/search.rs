//! Поиск, ссылки и уведомления.

use dimaster_grid::{Point, Size, Terminal, search_pattern};

fn run(cols: usize, input: &str) -> Terminal {
    let mut term = Terminal::new(Size { rows: 3, cols });
    term.advance(input.as_bytes());
    term
}

#[test]
fn finds_across_history_and_wraps_with_smart_case() {
    let term = run(5, "Error\r\nab\r\ncd\r\nxxxerrorx");
    let found = term.search(&search_pattern("error", false).unwrap());
    assert_eq!(found.len(), 2);
    assert_eq!(found[1].start, Point { line: 3, col: 3 });
    assert_eq!(found[1].end, Point { line: 4, col: 2 });
    assert_eq!(
        term.search(&search_pattern("Error", false).unwrap()).len(),
        1
    );
    assert_eq!(term.search(&search_pattern("c.", true).unwrap()).len(), 1);
    assert!(
        term.search(&search_pattern("c.", false).unwrap())
            .is_empty()
    );
    assert!(search_pattern("(", true).is_none());
}

#[test]
fn detects_links_and_paths() {
    let term = run(60, "see https://example.com/a?b=1. and ~/x/y.rs:3");
    let link = |col| term.link_at(Point { line: 0, col }).map(|l| l.1);
    assert_eq!(link(6).unwrap(), "https://example.com/a?b=1");
    assert_eq!(link(37).unwrap(), "~/x/y.rs:3");
    assert_eq!(link(1), None);
}

#[test]
fn collects_notifications() {
    let mut term = run(
        10,
        "\x1b]9;done\x07\x1b]777;notify;Build;ok\x07\x1b]9;4;1;50\x07",
    );
    let expected = [("DimaSter", "done"), ("Build", "ok")];
    let got = term.take_notifications();
    let got: Vec<_> = got.iter().map(|(t, b)| (t.as_str(), b.as_str())).collect();
    assert_eq!(got, expected);
}
