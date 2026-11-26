//! Перевод сообщений DimaSter.
//!
//! Ключ перевода — сама русская строка: код читается без справочника ключей,
//! а текст, заданный пользователем в конфиге, в таблице не найдётся и
//! пройдёт без изменений. Английский встроен, остальные языки пользователь
//! добавляет файлами `lang/<код>.lua`.

use std::collections::HashMap;
use std::sync::RwLock;

// строки вида «русский<TAB>english»
const ENGLISH: &str = include_str!("en.tsv");

static TABLE: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

/// Пары «оригинал — перевод» встроенного английского.
pub fn english() -> impl Iterator<Item = (&'static str, &'static str)> {
    ENGLISH.lines().filter_map(|line| line.split_once('\t'))
}

/// Код языка по настройке; `auto` — из переменных окружения локали.
pub fn resolve(setting: &str, variable: impl Fn(&str) -> Option<String>) -> String {
    if setting != "auto" {
        return setting.to_owned();
    }
    let locale = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|name| variable(name).filter(|v| !v.is_empty()))
        .unwrap_or_default();
    let code: String = locale
        .chars()
        .take_while(char::is_ascii_alphabetic)
        .collect();
    match code.as_str() {
        // без локали остаётся язык оригинала
        "" | "C" | "POSIX" => "ru".to_owned(),
        _ => code,
    }
}

// чужой язык опирается на английский: непереведённое не станет русским
fn table(code: &str, user: Vec<(String, String)>) -> Option<HashMap<String, String>> {
    let mut table: HashMap<String, String> = match code {
        "ru" => HashMap::new(),
        _ => english()
            .map(|(a, b)| (a.to_owned(), b.to_owned()))
            .collect(),
    };
    table.extend(user);
    (!table.is_empty()).then_some(table)
}

/// Выбирает язык; `user` — переводы из файла пользователя.
pub fn set_language(setting: &str, user: Vec<(String, String)>) {
    let code = resolve(setting, |name| std::env::var(name).ok());
    // отравленный замок не опасен: таблица заменяется целиком
    *TABLE.write().unwrap_or_else(|e| e.into_inner()) = table(&code, user);
}

/// Перевод строки; без перевода — она сама.
pub fn tr(text: &str) -> String {
    let table = TABLE.read().unwrap_or_else(|e| e.into_inner());
    let found = table.as_ref().and_then(|t| t.get(text));
    found.map_or(text, String::as_str).to_owned()
}

/// Перевод с подстановкой значений вместо `{имя}`.
pub fn trf(text: &str, values: &[(&str, &str)]) -> String {
    values.iter().fold(tr(text), |text, (name, value)| {
        text.replace(&format!("{{{name}}}"), value)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_comes_from_locale() {
        let locale = |value: &'static str| move |name: &str| (name == "LANG").then(|| value.into());
        assert_eq!(resolve("auto", locale("de_DE.UTF-8")), "de");
        assert_eq!(resolve("auto", locale("C")), "ru");
        assert_eq!(resolve("en", locale("ru_RU.UTF-8")), "en");
    }

    #[test]
    fn user_words_win_and_unknown_pass() {
        let german = table("de", vec![("Копировать".into(), "Kopieren".into())]).unwrap();
        assert_eq!(german["Копировать"], "Kopieren");
        assert_eq!(german["Вставить"], "Paste");
        assert!(table("ru", Vec::new()).is_none());
        assert_eq!(trf("{a}+{a} {b}", &[("a", "1")]), "1+1 {b}");
    }
}
