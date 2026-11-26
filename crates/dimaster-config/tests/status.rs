//! Строка статуса: встроенный сценарий, своя форма и ошибки.

use dimaster_config::{BarStyle, Plugins, Segment, Snapshot, StatusBar};

fn facts(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect()
}

fn render(name: &str, script: Option<&str>, info: &[(&str, &str)]) -> StatusBar {
    let directory =
        std::env::temp_dir().join(format!("dimaster-status-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    if let Some(script) = script {
        std::fs::write(directory.join("statusbar.lua"), script).unwrap();
    }
    let (plugins, errors) = Plugins::load(&directory).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    let bar = plugins.status(&facts(info), Snapshot::default()).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    bar
}

fn text(side: &[Segment]) -> String {
    side.iter().map(|s| s.text.clone()).collect()
}

#[test]
fn builtin_script_shows_only_known_facts() {
    let info = [("mode", "NORMAL"), ("host", "h"), ("git_branch", "main")];
    let bar = render("builtin", None, &info);
    assert_eq!((bar.style, bar.rows.len()), (BarStyle::default(), 1));
    let line = &bar.rows[0];
    assert!(text(&line.left).contains("NORMAL"));
    assert!(text(&line.center).contains("main") && !text(&line.center).contains("node"));
    assert!(line.right.len() == 1 && line.left[0].color.is_some());
    assert_eq!(line.left[0].left_cap, "\u{e0b6}");
}

const SHAPE: &str = "dimaster.statusbar(function() return {
        position = 'top', margin = { x = 30, y = 6 }, padding = 8, gap = 2,
        background = '#101010', radius = 14, opacity = 0.5,
        rows = {
          { left = { { text = 'a', background = '#abcdef', left_cap = '<', right_cap = '' } } },
          { right = { { text = 'b', radius = 9, opacity = 7 }, 'c', 5 } },
        },
    } end)";

#[test]
fn shape_position_and_several_rows() {
    let bar = render("shape", Some(SHAPE), &[]);
    let expected = BarStyle {
        top: true,
        margin: (30, 6),
        padding: (8, 8),
        gap: 2,
        background: Some(0x101010),
        radius: 14,
        opacity: 0.5,
    };
    assert_eq!((bar.style, bar.rows.len()), (expected, 2));
    let first = &bar.rows[0].left[0];
    assert_eq!(
        (first.left_cap.as_str(), first.right_cap.as_str()),
        ("<", "")
    );
    let second = &bar.rows[1].right;
    assert_eq!(
        (second.len(), second[0].radius, second[0].opacity),
        (2, 9, 1.0)
    );
}

#[test]
fn huge_values_are_limited() {
    let script = "dimaster.statusbar(function() return { margin = 100000, radius = 100000 } end)";
    let bar = render("limit", Some(script), &[]);
    assert_eq!((bar.style.margin, bar.style.radius), ((200, 200), 200));
}
