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

// блоки ```lua, начинающиеся с `return {` или `local hour`
fn settings_examples(page: &str) -> impl Iterator<Item = &str> {
    page.split("```lua\n").skip(1).filter_map(|rest| {
        let block = &rest[..rest.find("\n```")?];
        let code = block.lines().find(|l| !l.starts_with("--"))?;
        let config = code.starts_with("return {") || code.starts_with("local hour");
        config.then_some(block)
    })
}

// темы, fetch и свои модули — не настройки или требуют каталога
const EXAMPLE_PAGES: [&str; 12] = [
    "interface/ui.md",
    "start.md",
    "recipes.md",
    "core/layers.md",
    "core/profiles.md",
    "settings/appearance.md",
    "settings/font.md",
    "settings/window.md",
    "settings/effects.md",
    "settings/behavior.md",
    "settings/programs.md",
    "interface/cursor.md",
];

#[test]
fn settings_examples_load() {
    let mut checked = 0;
    for page in EXAMPLE_PAGES {
        let text = std::fs::read_to_string(format!("../../docs/lua/{page}")).unwrap();
        for block in settings_examples(&text) {
            let source = block.replace("dimaster.ssh", "false");
            let known = source.contains("theme = \"mine\"") || source.contains("require(");
            if !known {
                parse(&source, None).unwrap_or_else(|e| panic!("{page}: {e}\n{block}"));
                checked += 1;
            }
        }
    }
    assert!(checked > 20, "проверено примеров: {checked}");
}
