//! Каждый стиль загружается как настройки и даёт строку статуса.

use dimaster_config::{Plugins, Snapshot, Theme, parse_with, style_bar, styles};

#[test]
fn every_style_loads_and_draws_its_bar() {
    let directory = std::env::temp_dir().join(format!("dimaster-style-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let (plugins, _) = Plugins::load(&directory).unwrap();
    assert_eq!(styles().len(), 8);
    for style in styles() {
        let source = "return { cursor = { shape = \"bar\" }, scrollback = 5 }";
        let config = parse_with(source, None, &[style.literal.to_owned()]).unwrap();
        assert_eq!(config.scrollback, 5, "{}", style.id);
        let theme = Theme::default();
        plugins.set_bar(Some(&style_bar(style, &theme))).unwrap();
        let facts = [("mode".to_owned(), "NORMAL".to_owned())];
        let bar = plugins.status(&facts, Snapshot::default()).unwrap();
        let left = &bar.rows[0].left;
        assert!(
            left.iter().any(|s| s.text.contains("NORMAL")),
            "{}",
            style.id
        );
    }
    plugins.set_bar(None).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
}
