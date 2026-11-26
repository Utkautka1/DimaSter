//! Каждая строка, переданная в перевод, есть в английской таблице.

use std::path::Path;

// строковый литерал сразу после `tr(`, `trf(` или `t!(`
fn keys(source: &str) -> Vec<String> {
    let code = source.split("#[cfg(test)]").next().unwrap_or_default();
    let calls = ["tr(", "trf(", "t!("]
        .into_iter()
        .flat_map(|c| code.match_indices(c))
        // `t!(` входит и в `format!(`: нужен отдельный вызов
        .filter(|(at, _)| !code[..*at].ends_with(|c: char| c.is_alphanumeric() || c == '_'));
    let literals = calls.filter_map(|(at, call)| {
        let rest = code[at + call.len()..].trim_start().strip_prefix('"')?;
        Some(rest[..rest.find('"')?].to_owned())
    });
    literals
        .filter(|t| t.chars().any(|c| ('а'..='я').contains(&c)))
        .collect()
}

fn sources(directory: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(directory).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.extend(keys(&std::fs::read_to_string(path).unwrap()));
        }
    }
}

#[test]
fn every_translated_literal_has_english() {
    let known: Vec<&str> = dimaster_i18n::english().map(|pair| pair.0).collect();
    let mut used = Vec::new();
    for entry in std::fs::read_dir("..").unwrap().flatten() {
        let source = entry.path().join("src");
        if source.is_dir() && entry.file_name() != "dimaster-i18n" {
            sources(&source, &mut used);
        }
    }
    let missing: Vec<_> = used
        .iter()
        .filter(|k| !known.contains(&k.as_str()))
        .collect();
    assert!(
        used.len() > 20 && missing.is_empty(),
        "нет перевода: {missing:?}"
    );
}
