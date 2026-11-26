//! Совместимость с программами: цвета, буфер обмена, запросы, заголовки.

use dimaster_grid::{Size, Terminal};

fn run(input: &str) -> Terminal {
    let mut term = Terminal::new(Size { rows: 5, cols: 20 });
    term.advance(input.as_bytes());
    term
}

fn reply(input: &str) -> String {
    String::from_utf8(run(input).take_reply()).unwrap()
}

#[test]
fn programs_change_and_reset_colors() {
    let mut term = run("\x1b]4;1;#ff0000;200;rgb:0/f/0\x07\x1b]11;#102030\x07\x1b]12;#fff\x07");
    let colors = term.color_overrides();
    assert_eq!(
        (colors.palette[&1], colors.palette[&200]),
        (0xff0000, 0x00ff00)
    );
    assert_eq!(
        (colors.background, colors.cursor),
        (Some(0x102030), Some(0xffffff))
    );
    term.advance(b"\x1b]104;1\x07\x1b]111\x07");
    assert_eq!(term.color_overrides().palette.len(), 1);
    assert_eq!(term.color_overrides().background, None);
    assert_eq!(
        reply("\x1b]4;1;#010203\x07\x1b]4;1;?\x07"),
        "\x1b]4;1;rgb:0101/0202/0303\x1b\\"
    );
}

#[test]
fn osc52_writes_clipboard_but_never_reads() {
    let mut term = run("\x1b]52;c;aGVsbG8=\x07\x1b]52;p;eA==\x07\x1b]52;c;?\x07");
    let writes = term.take_clipboard_writes();
    assert_eq!(
        writes,
        [(false, "hello".to_owned()), (true, "x".to_owned())]
    );
    assert!(term.take_reply().is_empty());
}

#[test]
fn answers_settings_and_capabilities() {
    assert_eq!(
        reply("\x1b[1;31m\x1bP$qm\x1b\\"),
        "\x1bP1$r0;1;38:5:1m\x1b\\"
    );
    assert_eq!(reply("\x1b[2;4r\x1bP$qr\x1b\\"), "\x1bP1$r2;4r\x1b\\");
    assert_eq!(reply("\x1bP+q436f\x1b\\"), "\x1bP1+r436f=323536\x1b\\");
    assert_eq!(reply("\x1bP+q7a7a\x1b\\"), "\x1bP0+r7a7a\x1b\\");
}

#[test]
fn title_stack_and_soft_reset() {
    let term = run("\x1b]0;a\x07\x1b[22;0t\x1b]0;b\x07\x1b[23;0t");
    assert_eq!(term.title(), "a");
    let term = run("keep\x1b[4h\x1b[?25l\x1b[!p");
    assert_eq!(term.text(), "keep");
    assert!(term.modes().cursor_visible && !term.modes().insert);
}

#[test]
fn jumps_between_prompts() {
    let prompt = "\x1b]133;A\x07$ \r\n1\r\n2\r\n3\r\n4\r\n";
    let term = run(&format!("{prompt}{prompt}"));
    let last = term.line_range().end - 1;
    let second = term.prompt_line(last, false).unwrap();
    let first = term.prompt_line(second, false).unwrap();
    assert!(first < second);
    assert_eq!(term.prompt_line(first, true), Some(second));
    assert_eq!(term.prompt_line(first, false), None);
}
