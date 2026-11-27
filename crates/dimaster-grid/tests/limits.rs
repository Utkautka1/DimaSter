//! Пределы памяти и то, что обязано переживать сброс.

use dimaster_grid::{Point, Size, Terminal};

fn term() -> Terminal {
    Terminal::new(Size { rows: 4, cols: 20 })
}

// нулевые пиксели RGBA со стороной `side` в base64
fn pixels(side: usize) -> String {
    "A".repeat((side * side * 4).div_ceil(3) * 4)
}

fn link(t: &Terminal, line: u64) -> Option<String> {
    t.link_at(Point { line, col: 0 }).map(|l| l.1)
}

fn print_link(t: &mut Terminal, uri: &str) {
    t.advance(format!("\r\x1b]8;;{uri}\x07x\x1b]8;;\x07").as_bytes());
}

#[test]
fn reset_keeps_history_links() {
    let mut t = term();
    print_link(&mut t, "https://first.example");
    t.advance("\r\n".repeat(6).as_bytes());
    let line = t.line_range().start;
    t.advance(b"\x1bc");
    print_link(&mut t, "https://second.example");
    assert_eq!(link(&t, line).as_deref(), Some("https://first.example"));
}

#[test]
fn forgotten_link_numbers_are_reused() {
    let mut t = term();
    t.set_history_limit(10);
    for n in 0..6000 {
        print_link(&mut t, &format!("https://a.example/{n}"));
        t.advance(b"\r\n");
    }
    print_link(&mut t, "https://last.example");
    let line = t.visible_line(0) + t.cursor().row as u64;
    assert_eq!(link(&t, line).as_deref(), Some("https://last.example"));
}

#[test]
fn animation_frames_are_bounded() {
    let mut t = term();
    let data = pixels(512);
    t.advance(format!("\x1b_Ga=T,i=1,f=32,s=512,v=512,q=2;{data}\x1b\\").as_bytes());
    for _ in 0..400 {
        t.advance(b"\x1b_Ga=f,i=1,f=32,s=1,v=1,q=2;AAAAAA==\x1b\\");
    }
    let images = t.screen().rows().iter().flat_map(|r| &r.images);
    assert!(images.map(|p| p.picture.bytes()).all(|b| b <= 256 << 20));
}

#[test]
fn virtual_pictures_share_the_limit() {
    let mut t = term();
    let data = pixels(512);
    for id in 1..=300 {
        let command = format!("\x1b_Ga=T,U=1,i={id},f=32,s=512,v=512,q=2;{data}\x1b\\");
        t.advance(command.as_bytes());
    }
    assert!(t.virtual_placement(1, 0).is_none());
    assert!(t.virtual_placement(300, 0).is_some());
}

#[test]
fn absurd_size_does_not_overflow() {
    let gif = "R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";
    term().advance(format!("\x1b_Da=show,c={};{gif}\x1b\\", u64::MAX).as_bytes());
}

#[test]
fn parsing_pauses_for_external_decoder() {
    let mut t = term();
    // «xxxx» — не PNG и не GIF, значит нужен внешний декодер
    let input = b"\x1b_Da=show;eHh4eA==\x1b\\after";
    let (used, slow) = t.advance_part(input);
    assert!(slow.is_some() && t.text().is_empty());
    assert_eq!(&input[used..], b"after");
}
