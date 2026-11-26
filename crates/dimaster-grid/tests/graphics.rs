//! Графические протоколы: размещение, прокрутка, удаление и ответы.

use dimaster_grid::{Size, Terminal};
use proptest::prelude::*;

fn run(rows: usize, cols: usize, input: &str) -> Terminal {
    let mut term = Terminal::new(Size { rows, cols });
    term.advance(input.as_bytes());
    term
}

// (строка экрана, колонка, ширина, высота) каждой видимой картинки
fn images(term: &Terminal) -> Vec<(usize, usize, usize, usize)> {
    let rows = term.screen().rows().iter().enumerate();
    rows.flat_map(|(y, r)| r.images.iter().map(move |p| (y, p.col, p.cols, p.rows)))
        .collect()
}

fn history_images(term: &Terminal) -> usize {
    term.screen().history().map(|r| r.images.len()).sum()
}

// 20x40 пикселей RGBA = 2x2 ячейки по 10x20
fn rgba(keys: &str) -> String {
    let data = "/wAA".repeat(20 * 40 * 4 / 3 + 1);
    format!("\x1b_G{keys},f=32,s=20,v=40;{data}\x1b\\")
}

#[test]
fn kitty_places_image_and_moves_cursor() {
    let mut term = run(6, 20, &rgba("a=T,i=7"));
    assert_eq!(images(&term), [(0, 0, 2, 2)]);
    assert_eq!((term.cursor().row, term.cursor().col), (1, 2));
    assert_eq!(term.take_reply(), b"\x1b_Gi=7;OK\x1b\\");
}

#[test]
fn kitty_chunks_put_and_delete() {
    let full = rgba("a=t,i=3,q=2");
    let (header, data) = full[3..full.len() - 2].split_once(';').unwrap();
    let (first, second) = data.split_at(400);
    let mut term = run(6, 20, &format!("\x1b_G{header},m=1;{first}\x1b\\"));
    term.advance(format!("\x1b_Gm=0;{second}\x1b\\").as_bytes());
    assert!(images(&term).is_empty());
    term.advance(b"\x1b_Ga=p,i=3,c=4,C=1,q=2\x1b\\");
    assert_eq!(images(&term), [(0, 0, 4, 4)]);
    term.advance(b"\x1b_Ga=d,d=i,i=3\x1b\\");
    assert!(images(&term).is_empty());
}

#[test]
fn query_answers_without_showing() {
    let mut term = run(3, 10, "\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[c");
    assert_eq!(term.take_reply(), b"\x1b_Gi=31;OK\x1b\\\x1b[?62;4;22c");
    assert!(images(&term).is_empty());
}

#[test]
fn images_scroll_into_history_unless_disabled() {
    let mut term = run(3, 20, &rgba("a=T"));
    term.advance(b"\r\n\n\n\n");
    assert!(images(&term).is_empty());
    assert_eq!(history_images(&term), 1);
    let mut off = run(3, 20, "");
    off.set_images_scroll(false);
    off.advance(format!("{}\r\n\n\n\n", rgba("a=T")).as_bytes());
    assert_eq!(history_images(&off), 0);
}

#[test]
fn clear_screen_and_reflow() {
    let mut term = run(6, 20, &format!("ab{}", rgba("a=T")));
    term.resize(Size { rows: 6, cols: 10 });
    assert_eq!(images(&term), [(0, 2, 2, 2)]);
    term.advance(b"\x1b[2J");
    assert!(images(&term).is_empty());
}

#[test]
fn sixel_iterm_and_own_protocol() {
    let sixel = run(6, 20, "\x1bPq#1;2;100;0;0#1!20~\x1b\\");
    assert_eq!(images(&sixel), [(0, 0, 2, 1)]);
    assert_eq!(sixel.cursor().row, 1);
    // PNG 1x1
    let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhWpmIQAAAABJRU5ErkJggg==";
    let iterm = run(
        6,
        20,
        &format!("\x1b]1337;File=inline=1;width=3;height=2:{png}\x07"),
    );
    assert_eq!(images(&iterm), [(0, 0, 3, 2)]);
    let own = run(6, 20, &format!("\x1b_Da=show,c=4;{png}\x1b\\"));
    assert_eq!(images(&own), [(0, 0, 4, 2)]);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    #[test]
    fn garbage_graphics_do_not_panic(
        kind in "(_G|_D|Pq|\\]1337;File=)",
        body in "[a-zA-Z0-9=,;:#!$\\-+/]{0,80}",
        cols in 2usize..12,
    ) {
        let mut term = run(3, cols, &format!("\x1b{kind}{body}\x1b\\x"));
        term.resize(Size { rows: 2, cols: 3 });
        prop_assert!(term.cursor().col < 3);
    }
}
