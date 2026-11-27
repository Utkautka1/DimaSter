//! Таблица `tuning`: тайминги и пределы поведения окна.

use mlua::Table;

/// Тонкая настройка поведения.
#[derive(Debug, Clone, PartialEq)]
pub struct Tuning {
    // миллисекунды
    pub multi_click: u64,
    pub drag_scroll_step: u64,
    pub double_alt: u64,
    pub glow_period: u64,
    pub bell_flash: u64,
    // секунды
    pub autosave: u64,
    // сколько команд и каталогов хранится на диске
    pub history_commands: usize,
    pub history_dirs: usize,
    // пределы выбора файлов
    pub picker_files: usize,
    pub picker_depth: usize,
    pub picker_skip: Vec<String>,
    /// Буквы меток на ссылках.
    pub hint_letters: String,
    /// Пары автозакрытия.
    pub pairs: Vec<(char, char)>,
    /// На сколько пунктов крупнее шрифт в презентации.
    pub presentation_zoom: f32,
    /// Строка под восстановленной сессией.
    pub restored_marker: String,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            multi_click: 400,
            drag_scroll_step: 45,
            double_alt: 350,
            glow_period: 1600,
            bell_flash: 150,
            autosave: 30,
            history_commands: 1000,
            history_dirs: 300,
            picker_files: 1500,
            picker_depth: 5,
            picker_skip: [".git", "target", "node_modules", ".cache"]
                .map(str::to_owned)
                .to_vec(),
            hint_letters: "asdfghjklqwertyuiopzxcvbnm".to_owned(),
            pairs: vec![('(', ')'), ('[', ']'), ('{', '}'), ('"', '"'), ('\'', '\'')],
            presentation_zoom: 4.0,
            restored_marker: "── сессия восстановлена ──".to_owned(),
        }
    }
}

fn set<T: mlua::FromLua>(table: &Table, key: &str, target: &mut T) -> mlua::Result<()> {
    if let Some(value) = table.get::<Option<T>>(key)? {
        *target = value;
    }
    Ok(())
}

fn timings(table: &Table, tuning: &mut Tuning) -> mlua::Result<()> {
    set(table, "multi_click", &mut tuning.multi_click)?;
    set(table, "drag_scroll_step", &mut tuning.drag_scroll_step)?;
    set(table, "double_alt", &mut tuning.double_alt)?;
    set(table, "glow_period", &mut tuning.glow_period)?;
    set(table, "bell_flash", &mut tuning.bell_flash)?;
    set(table, "autosave", &mut tuning.autosave)?;
    // нулевые значения зациклили бы окно
    tuning.drag_scroll_step = tuning.drag_scroll_step.max(10);
    tuning.glow_period = tuning.glow_period.max(100);
    tuning.autosave = tuning.autosave.max(5);
    Ok(())
}

fn limits(table: &Table, tuning: &mut Tuning) -> mlua::Result<()> {
    set(table, "history_commands", &mut tuning.history_commands)?;
    set(table, "history_dirs", &mut tuning.history_dirs)?;
    set(table, "picker_files", &mut tuning.picker_files)?;
    set(table, "picker_depth", &mut tuning.picker_depth)?;
    set(table, "picker_skip", &mut tuning.picker_skip)?;
    set(table, "presentation_zoom", &mut tuning.presentation_zoom)?;
    set(table, "restored_marker", &mut tuning.restored_marker)
}

// "()" → ('(', ')'); не ASCII shell получил бы искажённым
fn pair(text: &str) -> Option<(char, char)> {
    match text.chars().collect::<Vec<_>>()[..] {
        [open, close] if open.is_ascii() && close.is_ascii() => Some((open, close)),
        _ => None,
    }
}

// меньше двух разных букв метки не различить
fn letters(text: &str) -> Option<String> {
    let mut found: Vec<char> = Vec::new();
    for ch in text.chars().map(|c| c.to_ascii_lowercase()) {
        if ch.is_ascii_alphanumeric() && !found.contains(&ch) {
            found.push(ch);
        }
    }
    (found.len() > 1).then(|| found.into_iter().collect())
}

pub(crate) fn apply(table: &Table, tuning: &mut Tuning) -> mlua::Result<()> {
    timings(table, tuning)?;
    limits(table, tuning)?;
    if let Some(texts) = table.get::<Option<Vec<String>>>("pairs")? {
        tuning.pairs = texts.iter().filter_map(|t| pair(t)).collect();
    }
    if let Some(found) = table.get::<Option<String>>("hint_letters")? {
        tuning.hint_letters = letters(&found).unwrap_or_else(|| tuning.hint_letters.clone());
    }
    Ok(())
}
