//! Таблица `ui`: разбор и соответствие примера значениям по умолчанию.

use dimaster_config::{Anchor, Config, WindowButton, parse};

const EXAMPLE: &str = include_str!("../src/scaffold/files/ui.lua");

// строки «-- ключ = значение» превращаются в настоящие
fn uncommented() -> String {
    let lines = EXAMPLE.lines().map(|line| {
        let trimmed = line.trim_start();
        match trimmed.strip_prefix("-- ") {
            Some(rest) if rest.starts_with(|c: char| c.is_ascii_lowercase()) => rest,
            _ => line,
        }
    });
    lines.collect::<Vec<_>>().join("\n")
}

#[test]
fn example_lists_real_defaults() {
    let config = parse(&uncommented(), None).unwrap();
    let mut expected = Config::default().ui;
    // цвета темы и свой список палитры в примере — иллюстрации
    expected.panel = config.ui.panel;
    expected.title.background = config.ui.title.background;
    expected.title.color = config.ui.title.color;
    expected.palette.items = config.ui.palette.items.clone();
    assert_eq!(config.ui, expected);
}

#[test]
fn values_are_read_and_clamped() {
    let source = r##"return { ui = {
        panel = { radius = 999, background = "#102030" },
        palette = { position = "center", width = 1 },
        title = { buttons = { { action = "close", text = "x" } } },
    } }"##;
    let ui = parse(source, None).unwrap().ui;
    assert_eq!(
        (ui.panel.radius, ui.panel.background),
        (200, Some(0x102030))
    );
    assert_eq!(
        (ui.palette.position, ui.palette.width),
        (Anchor::Center, 10)
    );
    assert_eq!(ui.title.buttons[0].kind, WindowButton::Close);
}

#[test]
fn wrong_word_is_reported() {
    let source = r#"return { ui = { title = { align = "middle" } } }"#;
    assert!(parse(source, None).is_err());
}
