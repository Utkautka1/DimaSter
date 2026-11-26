//! Запись настроек обратно в Lua — так мастер создаёт `init.lua`.

use std::fmt::Write;

use crate::color::format_color;
use crate::{Config, Theme};

// строка Lua: экранируются только символы, ломающие литерал
fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' | '\\' => out.extend(['\\', ch]),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
    out + "\""
}

fn colors(theme: &Theme) -> String {
    let hex = |c| quote(&format_color(c));
    let selection_text = theme
        .selection_text
        .map(|c| format!("selection_text = {},\n    ", hex(c)));
    let ansi: Vec<_> = theme.ansi.iter().map(|&c| hex(c)).collect();
    format!(
        "  colors = {{\n    foreground = {},\n    background = {},\n    cursor = {},\n    \
         selection = {},\n    {}ansi = {{\n      {},\n      {},\n    }},\n  }},\n",
        hex(theme.foreground),
        hex(theme.background),
        hex(theme.cursor),
        hex(theme.selection),
        selection_text.unwrap_or_default(),
        ansi[..8].join(", "),
        ansi[8..].join(", "),
    )
}

/// Полный конфиг с комментарием-подсказкой в начале.
pub fn to_lua(config: &Config) -> String {
    let mut out = String::from(
        "-- Настройки DimaSter. Сохраните файл — изменения применятся сразу.\n\
         -- Темы: theme = \"nord\" (или свой файл themes/<имя>.lua вместо colors).\n\
         -- Клавиши: keys = { { key = \"t\", mods = \"ctrl+shift\", action = \"scroll_top\" } }\n\
         return {\n",
    );
    let family = config
        .font_family
        .as_ref()
        .map(|f| format!("family = {}, ", quote(f)));
    let _ = writeln!(
        out,
        "  font = {{ {}size = {} }},",
        family.unwrap_or_default(),
        config.font_size
    );
    let (x, y) = config.padding;
    let _ = writeln!(out, "  padding = {{ x = {x}, y = {y} }},");
    let _ = writeln!(out, "  cursor = {},", quote(config.cursor.name()));
    let _ = writeln!(out, "  scrollback = {},", config.scrollback);
    if !config.shell.is_empty() {
        let words: Vec<_> = config.shell.iter().map(|w| quote(w)).collect();
        let _ = writeln!(out, "  shell = {{ {} }},", words.join(", "));
    }
    out + &colors(&config.theme) + "}\n"
}
