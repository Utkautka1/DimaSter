//! Каждый стиль загружается как настройки, а бары различаются составом и местом.

use dimaster_config::{Plugins, Snapshot, StatusBar, Style, Theme, parse_with, style_bar, styles};

fn facts() -> Vec<(String, String)> {
    let known = [
        ("mode", "NORMAL"),
        ("host", "h"),
        ("user", "u"),
        ("cwd", "/tmp/x"),
        ("git_branch", "main"),
    ];
    known.map(|(k, v)| (k.to_owned(), v.to_owned())).to_vec()
}

fn bar(plugins: &Plugins, style: &Style) -> StatusBar {
    let theme = Theme::default();
    plugins.set_bar(Some(&style_bar(style, &theme))).unwrap();
    plugins.status(&facts(), Snapshot::default()).unwrap()
}

fn shape(bar: &StatusBar) -> String {
    let rows = bar.rows.iter().map(|row| {
        let side =
            |s: &[dimaster_config::Segment]| s.iter().map(|x| x.text.as_str()).collect::<String>();
        format!(
            "{}|{}|{}",
            side(&row.left),
            side(&row.center),
            side(&row.right)
        )
    });
    format!("{} {}", bar.style.top, rows.collect::<Vec<_>>().join("/"))
}

#[test]
fn styles_differ_in_widgets_and_layout() {
    let directory = std::env::temp_dir().join(format!("dimaster-style-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let (plugins, _) = Plugins::load(&directory).unwrap();
    let mut shapes = Vec::new();
    for style in styles() {
        let config = parse_with("return {}", None, &[style.literal.to_owned()]).unwrap();
        assert!(style.prompt.contains("[character]"), "{}", style.id);
        if config.widgets.statusbar {
            shapes.push(shape(&bar(&plugins, style)));
        }
    }
    let count = shapes.len();
    shapes.sort();
    shapes.dedup();
    assert_eq!(
        (styles().len(), count, shapes.len()),
        (8, 6, 6),
        "{shapes:#?}"
    );
    std::fs::remove_dir_all(directory).unwrap();
}
