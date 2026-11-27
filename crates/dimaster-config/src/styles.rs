//! Стили витрины: цельный вид терминала поверх любой палитры.
//!
//! Стиль — это обычная таблица настроек, строка статуса и фон. Фон только
//! называется здесь, рисует его приложение.

use crate::Theme;

/// Какую картинку рисовать за текстом.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    None,
    Blobs,
    Grid,
    Scanlines,
    Waves,
    Sunset,
    City,
    Petals,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Style {
    pub id: &'static str,
    pub title: &'static str,
    /// Таблица настроек литералом Lua.
    pub literal: &'static str,
    pub scene: Scene,
    // заметность фона в процентах: текст должен читаться
    pub shade: u8,
}

macro_rules! style {
    ($id:literal, $title:literal, $scene:ident, $shade:literal) => {
        Style {
            id: $id,
            title: $title,
            literal: include_str!(concat!("styles/", $id, ".lua")),
            scene: Scene::$scene,
            shade: $shade,
        }
    };
}

static STYLES: [Style; 8] = [
    style!("glass", "Стекло", Blobs, 45),
    style!("neon", "Неон", Grid, 55),
    style!("minimal", "Минимализм", None, 0),
    style!("retro", "Ретро", Scanlines, 35),
    style!("mac", "macOS", Waves, 35),
    style!("sunset", "Закат", Sunset, 50),
    style!("city", "Ночной город", City, 55),
    style!("petals", "Лепестки", Petals, 55),
];

pub fn styles() -> &'static [Style] {
    &STYLES
}

pub fn style(id: &str) -> Option<&'static Style> {
    STYLES.iter().find(|s| s.id == id)
}

/// Сценарий строки статуса стиля с цветами `theme`.
pub fn style_bar(style: &Style, theme: &Theme) -> String {
    let hex = |color: u32| format!("\"#{color:06x}\"");
    let ansi: Vec<String> = theme.ansi.iter().map(|&c| hex(c)).collect();
    format!(
        "local STYLE = \"{}\"\nlocal THEME = {{ bg = {}, fg = {}, ansi = {{ {} }} }}\n{}",
        style.id,
        hex(theme.background),
        hex(theme.foreground),
        ansi.join(", "),
        include_str!("styles/bar.lua")
    )
}
