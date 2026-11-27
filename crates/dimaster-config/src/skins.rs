//! Встроенные скины витрины: 50 цветовых схем.
//!
//! Палитры взяты из набора iTerm2 Color Schemes (лицензия MIT, см.
//! `skins.LICENSE`). Формат строки: имя, табуляция, 21 цвет через пробел —
//! фон, текст, курсор, выделение, текст выделения и 16 цветов ANSI.

use crate::Theme;

const DATA: &str = include_str!("skins.tsv");

/// Готовая цветовая схема.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skin {
    pub name: &'static str,
    pub theme: Theme,
}

fn theme(colors: &str) -> Option<Theme> {
    let mut values = colors
        .split(' ')
        .map(|hex| u32::from_str_radix(hex, 16).ok());
    let mut next = || values.next().flatten();
    let mut theme = Theme {
        background: next()?,
        foreground: next()?,
        cursor: next()?,
        selection: next()?,
        selection_text: Some(next()?),
        ansi: [0; 16],
    };
    for slot in &mut theme.ansi {
        *slot = next()?;
    }
    Some(theme)
}

/// Все скины в порядке витрины.
pub fn skins() -> Vec<Skin> {
    let rows = DATA.lines().filter_map(|line| line.split_once('\t'));
    rows.filter_map(|(name, colors)| {
        Some(Skin {
            name,
            theme: theme(colors)?,
        })
    })
    .collect()
}

/// Литерал Lua с цветами скина — накладывается поверх настроек;
/// `None`, если такого скина нет.
pub fn skin_literal(name: &str) -> Option<String> {
    let skin = skins().into_iter().find(|s| s.name == name)?;
    Some(theme_literal(&skin.theme))
}

/// Палитра таблицей настроек `{ colors = ... }`.
pub fn theme_literal(theme: &Theme) -> String {
    let hex = |color: u32| format!("\"#{color:06x}\"");
    let ansi: Vec<String> = theme.ansi.iter().map(|&c| hex(c)).collect();
    format!(
        "{{ colors = {{ background = {}, foreground = {}, cursor = {}, selection = {}, \
         selection_text = {}, ansi = {{ {} }} }} }}",
        hex(theme.background),
        hex(theme.foreground),
        hex(theme.cursor),
        hex(theme.selection),
        hex(theme.selection_text.unwrap_or(theme.foreground)),
        ansi.join(", ")
    )
}
