//! Ключ `cursor`: строка или таблица.

use dimaster_config::{CursorStyle, parse};

#[test]
fn name_alone_and_full_table() {
    let plain = parse(r#"return { cursor = "bar" }"#, None).unwrap();
    assert_eq!(plain.cursor, CursorStyle::Bar);
    let source = r##"return { cursor = { shape = "underline", thickness = 4,
        color = "#ff0000", easing = "linear", speed = 1e9, opacity = 0 } }"##;
    let config = parse(source, None).unwrap();
    let look = config.cursor_look;
    assert_eq!(config.cursor, CursorStyle::Underline);
    assert_eq!(
        (look.thickness, look.color, look.linear),
        (4, Some(0xff0000), true)
    );
    assert_eq!((look.speed, look.opacity), (2000.0, 0.05));
    assert!(parse(r#"return { cursor = { easing = "x" } }"#, None).is_err());
}
