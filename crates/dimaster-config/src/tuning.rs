//! Таблица `tuning`: тайминги и пределы поведения окна.

use mlua::Table;

use crate::{Chord, Mods};

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
    /// Клавиши: принять всю подсказку.
    pub suggest_line: Vec<Chord>,
    /// Клавиши: принять слово или исправить опечатку.
    pub suggest_word: Vec<Chord>,
    // сколько команд и строк вывода каждой держать в памяти окна
    pub memory_commands: usize,
    pub memory_lines: usize,
    // пиксели
    pub minimap_margin: u32,
    pub resize_edge: u32,
    // видео в `icat`
    pub video_width: usize,
    pub video_fps: u32,
    // секунды ожидания `docker ps` для подсказок и статуса
    pub docker_timeout: u64,
}

// "ctrl+right" → клавиша и модификаторы; стрелки можно писать коротко
fn chord(text: &str) -> Option<Chord> {
    let text = text.trim().to_ascii_lowercase();
    let (mods, key) = text.rsplit_once('+').unwrap_or(("", &text));
    let key = match key {
        "left" | "right" | "up" | "down" => format!("arrow{key}"),
        "" => return None,
        _ => key.to_owned(),
    };
    Some(Chord {
        key,
        mods: Mods::parse(mods)?,
    })
}

pub(crate) fn chords(texts: &[&str]) -> Vec<Chord> {
    texts.iter().filter_map(|t| chord(t)).collect()
}

fn keys(table: &Table, key: &str, target: &mut Vec<Chord>) -> mlua::Result<()> {
    if let Some(texts) = table.get::<Option<Vec<String>>>(key)? {
        *target = texts.iter().filter_map(|t| chord(t)).collect();
    }
    Ok(())
}

fn sizes(table: &Table, tuning: &mut Tuning) -> mlua::Result<()> {
    set(table, "memory_commands", &mut tuning.memory_commands)?;
    set(table, "memory_lines", &mut tuning.memory_lines)?;
    set(table, "minimap_margin", &mut tuning.minimap_margin)?;
    set(table, "resize_edge", &mut tuning.resize_edge)?;
    set(table, "video_width", &mut tuning.video_width)?;
    set(table, "video_fps", &mut tuning.video_fps)?;
    set(table, "docker_timeout", &mut tuning.docker_timeout)?;
    clamp(tuning);
    Ok(())
}

// без этих границ видео и память вышли бы из-под контроля
fn clamp(tuning: &mut Tuning) {
    tuning.memory_commands = tuning.memory_commands.clamp(1, 10_000);
    tuning.memory_lines = tuning.memory_lines.min(10_000);
    tuning.video_width = tuning.video_width.clamp(16, 1920);
    tuning.video_fps = tuning.video_fps.clamp(1, 60);
    tuning.docker_timeout = tuning.docker_timeout.clamp(1, 60);
}

impl Default for Tuning {
    fn default() -> Self {
        let (multi_click, drag_scroll_step, double_alt) = (400, 45, 350);
        let (glow_period, bell_flash, autosave) = (1600, 150, 30);
        let (history_commands, history_dirs) = (1000, 300);
        let (picker_files, picker_depth, presentation_zoom) = (1500, 5, 4.0);
        let (memory_commands, memory_lines, docker_timeout) = (100, 200, 2);
        let (minimap_margin, resize_edge, video_width, video_fps) = (8, 6, 480, 15);
        #[rustfmt::skip]
        #[allow(clippy::let_and_return)] // атрибут форматирования нужен на выражении
        let tuning = Self {
            multi_click, drag_scroll_step, double_alt, glow_period, bell_flash, autosave,
            history_commands, history_dirs, picker_files, picker_depth, presentation_zoom,
            memory_commands, memory_lines, docker_timeout,
            minimap_margin, resize_edge, video_width, video_fps,
            picker_skip: [".git", "target", "node_modules", ".cache"].map(str::to_owned).to_vec(),
            hint_letters: "asdfghjklqwertyuiopzxcvbnm".to_owned(),
            pairs: vec![('(', ')'), ('[', ']'), ('{', '}'), ('"', '"'), ('\'', '\'')],
            restored_marker: "── сессия восстановлена ──".to_owned(),
            suggest_line: chords(&["right", "end"]),
            suggest_word: chords(&["ctrl+right", "ctrl+end", "ctrl+shift+right", "ctrl+shift+end"]),
        };
        tuning
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
    sizes(table, tuning)?;
    keys(table, "suggest_line", &mut tuning.suggest_line)?;
    keys(table, "suggest_word", &mut tuning.suggest_word)?;
    if let Some(texts) = table.get::<Option<Vec<String>>>("pairs")? {
        tuning.pairs = texts.iter().filter_map(|t| pair(t)).collect();
    }
    if let Some(found) = table.get::<Option<String>>("hint_letters")? {
        tuning.hint_letters = letters(&found).unwrap_or_else(|| tuning.hint_letters.clone());
    }
    Ok(())
}
