//! Сценарий fetch: встроенный и пользовательский.

use dimaster_config::{DEFAULT_FETCH, run_fetch};

fn facts() -> Vec<(&'static str, Option<String>)> {
    vec![
        ("user", Some("u".into())),
        ("host", Some("h".into())),
        ("os", Some("MyOS".into())),
        ("gpu", None),
    ]
}

#[test]
fn default_script_skips_unknown_facts() {
    let layout = run_fetch(DEFAULT_FETCH, None, &facts()).unwrap();
    let text = layout.lines.join("\n");
    assert!(text.contains("MyOS"));
    assert!(!text.contains("GPU"));
    assert_eq!(
        (layout.logo.len(), layout.logo_width, layout.image),
        (7, 22, None)
    );
}

#[test]
fn custom_script_and_errors() {
    let script = "return { lines = { info.os }, logo_width = 3 }";
    let layout = run_fetch(script, None, &facts()).unwrap();
    assert_eq!(
        (layout.lines, layout.logo_width),
        (vec!["MyOS".to_owned()], 3)
    );
    assert!(run_fetch("return 1", None, &facts()).is_err());
}
