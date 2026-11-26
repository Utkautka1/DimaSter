//! Внешний вид кадра: цвета, отступы и обработка RTL.

use std::collections::BTreeMap;
use std::sync::Arc;

use dimaster_grid::{ColorOverrides, Picture};

use crate::palette::Rgb;

/// Задаётся приложением из пользовательского конфига.
#[derive(Debug, Clone)]
pub struct Appearance {
    pub foreground: Rgb,
    pub background: Rgb,
    pub cursor: Rgb,
    pub selection: Rgb,
    pub selection_text: Option<Rgb>,
    pub ansi: [Rgb; 16],
    // отступы от краёв окна в пикселях: по горизонтали и вертикали
    pub padding: (u32, u32),
    pub rtl: bool,
    // непрозрачность фона окна
    pub opacity: f32,
    // высота своей полосы заголовка в пикселях, 0 — её нет
    pub top: u32,
    // фоновая картинка и насколько она заметна
    pub wallpaper: Option<(Arc<Picture>, f32)>,
    // цвета палитры, изменённые программой
    pub palette: BTreeMap<u8, Rgb>,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            foreground: 0xd8dee9,
            background: 0x11131a,
            cursor: 0xe5e9f0,
            selection: 0x3b4261,
            selection_text: None,
            ansi: [
                0x1d1f28, 0xf7768e, 0x9ece6a, 0xe0af68, 0x7aa2f7, 0xbb9af7, 0x7dcfff, 0xc0caf5,
                0x565f89, 0xff899d, 0xb9f27c, 0xffc777, 0x8db0ff, 0xc7a9ff, 0xa4daff, 0xe5e9f0,
            ],
            padding: (6, 6),
            rtl: true,
            opacity: 1.0,
            top: 0,
            wallpaper: None,
            palette: BTreeMap::new(),
        }
    }
}

impl Appearance {
    /// Тема с наложенными поверх изменениями от программы.
    pub(crate) fn with_overrides(&self, overrides: &ColorOverrides) -> Self {
        Self {
            foreground: overrides.foreground.unwrap_or(self.foreground),
            background: overrides.background.unwrap_or(self.background),
            cursor: overrides.cursor.unwrap_or(self.cursor),
            palette: overrides.palette.clone(),
            ..self.clone()
        }
    }
}
