//! Файлы настроек на другом языке не содержат русских строк.
//!
//! Язык общий на процесс, поэтому проверка живёт в отдельном тестовом файле.

use dimaster_config::{Config, scaffold};

fn cyrillic(line: &str) -> bool {
    line.chars()
        .any(|c| ('а'..='я').contains(&c) || ('А'..='Я').contains(&c))
}

#[test]
fn generated_files_are_fully_english() {
    dimaster_i18n::set_language("en", Vec::new());
    let mut imported = Config::default();
    imported.theme.background = 1;
    let left: Vec<String> = scaffold(&imported)
        .iter()
        .flat_map(|f| f.content.lines().filter(|l| cyrillic(l)).map(str::to_owned))
        .collect();
    assert!(left.is_empty(), "без перевода:\n{}", left.join("\n"));
}
