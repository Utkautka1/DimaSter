//! Справочник не должен отставать от кода.

use dimaster_config::{Action, Config, builtin_names, parse, scaffold};

const DOCS: &str = include_str!("../../../docs/LUA.md");

#[test]
fn every_action_and_theme_is_documented() {
    for name in Action::names().chain(builtin_names()) {
        assert!(DOCS.contains(&format!("`{name}`")), "не описано: {name}");
    }
}

#[test]
fn every_generated_setting_is_documented() {
    let generated: String = scaffold(&Config::default())
        .iter()
        .filter(|f| f.path.starts_with("settings/"))
        .map(|f| f.content.clone())
        .collect();
    let keys = generated.lines().filter_map(|line| {
        let line = line.trim_start().trim_start_matches("-- ");
        let (key, _) = line.split_once(" = ")?;
        key.chars()
            .all(|c| c.is_ascii_lowercase() || c == '_')
            .then_some(key)
    });
    for key in keys.filter(|k| *k != "key") {
        assert!(DOCS.contains(key), "не описано: {key}");
    }
}

// пример темы из справочника обязан загружаться
#[test]
fn documented_theme_example_is_valid() {
    let start = DOCS.find("return {\n  foreground").expect("пример темы");
    let end = start + DOCS[start..].find("\n```").unwrap();
    let colors = DOCS[start..end].replacen("return ", "return { colors = ", 1) + " }";
    parse(&colors, None).unwrap();
}
