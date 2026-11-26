//! Справочник не должен отставать от кода.

use dimaster_config::{Action, Config, builtin_names, parse, scaffold};

// все страницы docs/lua подряд
fn collect(directory: &std::path::Path, out: &mut String) {
    for entry in std::fs::read_dir(directory).unwrap().flatten() {
        match entry.path() {
            path if path.is_dir() => collect(&path, out),
            path => out.push_str(&std::fs::read_to_string(path).unwrap()),
        }
    }
}

fn docs() -> String {
    let mut out = String::new();
    collect(std::path::Path::new("../../docs/lua"), &mut out);
    out
}

#[test]
fn every_action_and_theme_is_documented() {
    let docs = docs();
    for name in Action::names().chain(builtin_names()) {
        assert!(docs.contains(&format!("`{name}`")), "не описано: {name}");
    }
}

#[test]
fn every_generated_setting_is_documented() {
    let docs = docs();
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
        assert!(docs.contains(key), "не описано: {key}");
    }
}

// пример темы из справочника обязан загружаться
#[test]
fn documented_theme_example_is_valid() {
    let docs = docs();
    let start = docs.find("return {\n  foreground").expect("пример темы");
    let end = start + docs[start..].find("\n```").unwrap();
    let colors = docs[start..end].replacen("return ", "return { colors = ", 1) + " }";
    parse(&colors, None).unwrap();
}
