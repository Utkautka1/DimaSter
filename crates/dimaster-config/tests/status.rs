//! Строка статуса: встроенный сценарий и пользовательский.

use dimaster_config::{Plugins, Snapshot};

fn facts(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
        .collect()
}

fn load(name: &str, script: Option<&str>) -> (Plugins, std::path::PathBuf) {
    let directory =
        std::env::temp_dir().join(format!("dimaster-status-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    if let Some(script) = script {
        std::fs::write(directory.join("statusbar.lua"), script).unwrap();
    }
    let (plugins, errors) = Plugins::load(&directory).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    (plugins, directory)
}

#[test]
fn builtin_script_shows_only_known_facts() {
    let (plugins, directory) = load("builtin", None);
    let info = facts(&[("mode", "NORMAL"), ("host", "h"), ("git_branch", "main")]);
    let line = plugins.status(&info, Snapshot::default()).unwrap();
    let text =
        |side: &[dimaster_config::Segment]| side.iter().map(|s| s.text.clone()).collect::<String>();
    assert!(text(&line.left).contains("NORMAL"));
    assert!(text(&line.center).contains("main") && !text(&line.center).contains("node"));
    assert!(line.right.len() == 1 && line.left[0].color.is_some());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn segments_can_be_pills() {
    let (plugins, directory) = load(
        "pill",
        Some(
            "dimaster.statusbar(function() return { left = { { text = 'u', background = '#abcdef', rounded = true } } } end)",
        ),
    );
    let line = plugins.status(&[], Snapshot::default()).unwrap();
    assert_eq!(
        (line.left[0].background, line.left[0].rounded),
        (Some(0xabcdef), true)
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn user_script_replaces_builtin_and_errors_are_contained() {
    let (plugins, directory) = load(
        "user",
        Some("dimaster.statusbar(function(i) return { left = { i.mode, 5 } } end)"),
    );
    let line = plugins
        .status(&facts(&[("mode", "VI")]), Snapshot::default())
        .unwrap();
    assert_eq!((line.left.len(), line.left[0].text.as_str()), (1, "VI"));
    assert!(plugins.status(&[], Snapshot::default()).is_some());
    std::fs::remove_dir_all(directory).unwrap();
}
