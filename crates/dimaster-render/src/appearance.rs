//! Внешний вид кадра: цвета, отступы и обработка RTL.

use std::collections::BTreeMap;
use std::sync::Arc;

use dimaster_grid::{ColorOverrides, Picture};

use crate::palette::Rgb;

/// Как выглядит курсор; доли — от размера ячейки.
#[derive(Debug, Clone, Copy)]
pub struct CursorLook {
    // 0 — толщина по шрифту
    pub thickness: u32,
    pub width: f32,
    pub height: f32,
    pub radius: u32,
    pub opacity: f32,
    pub color: Option<Rgb>,
    pub text_color: Option<Rgb>,
    pub hollow_unfocused: bool,
    pub trail_color: Option<Rgb>,
    pub trail_opacity: f32,
}

impl Default for CursorLook {
    fn default() -> Self {
        Self {
            thickness: 0,
            width: 1.0,
            height: 1.0,
            radius: 0,
            opacity: 1.0,
            color: None,
            text_color: None,
            hollow_unfocused: false,
            trail_color: None,
            trail_opacity: 0.35,
        }
    }
}

/// Задаётся приложением из пользовательского конфига.
#[derive(Debug, Clone)]
pub struct Appearance {
    pub foreground: Rgb,
    pub background: Rgb,
    pub cursor: Rgb,
    pub cursor_look: CursorLook,
    pub selection: Rgb,
    pub selection_text: Option<Rgb>,
    /// Цвет жирного текста без своего цвета; `None` — как обычный.
    pub bold: Option<Rgb>,
    pub ansi: [Rgb; 16],
    // отступы от краёв окна в пикселях: по горизонтали и вертикали
    pub padding: (u32, u32),
    pub rtl: bool,
    /// Минимальный контраст текста к фону (1.0 — выкл., до 21.0).
    pub minimum_contrast: f32,
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
            cursor_look: CursorLook::default(),
            selection: 0x3b4261,
            selection_text: None,
            bold: None,
            ansi: [
                0x1d1f28, 0xf7768e, 0x9ece6a, 0xe0af68, 0x7aa2f7, 0xbb9af7, 0x7dcfff, 0xc0caf5,
                0x565f89, 0xff899d, 0xb9f27c, 0xffc777, 0x8db0ff, 0xc7a9ff, 0xa4daff, 0xe5e9f0,
            ],
            padding: (6, 6),
            rtl: true,
            minimum_contrast: 1.0,
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
            // OSC 17 и 19 — выделение, OSC 5;0 — жирный текст
            selection: overrides
                .dynamic
                .get(&17)
                .copied()
                .unwrap_or(self.selection),
            selection_text: overrides.dynamic.get(&19).copied().or(self.selection_text),
            bold: overrides.special.get(&0).copied(),
            ..self.clone()
        }
    }
}
