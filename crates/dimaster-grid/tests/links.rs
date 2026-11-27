//! Что распознаётся под указателем.

use dimaster_grid::{Point, Size, Terminal, is_web_address};

fn at(text: &str, col: usize) -> (Option<String>, Option<String>) {
    let mut term = Terminal::new(Size { rows: 2, cols: 60 });
    term.advance(text.as_bytes());
    let point = Point { line: 0, col };
    (
        term.link_at(point).map(|l| l.1),
        term.word_at(point).map(|l| l.1),
    )
}

#[test]
fn bare_domains_are_links() {
    assert_eq!(at("curl example.com", 7).0.as_deref(), Some("example.com"));
    assert_eq!(
        at("see test.ru/a?b=1.", 6).0.as_deref(),
        Some("test.ru/a?b=1")
    );
    assert!(is_web_address("example.com:8080/x"));
    assert!(!is_web_address("main.rs") && !is_web_address("./a.com"));
}

#[test]
fn words_keep_escaped_spaces() {
    let (_, word) = at(r"cd Рабочий\ стол/terminal now", 5);
    assert_eq!(word.as_deref(), Some(r"Рабочий\ стол/terminal"));
    assert_eq!(at("src/main.rs:12", 1).1.as_deref(), Some("src/main.rs:12"));
}
