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
            foreground: 0xcdd6f4,
            background: 0x1e1e2e,
            cursor: 0xcba6f7,
            cursor_look: CursorLook::default(),
            selection: 0xf5e0dc,
            selection_text: Some(0x1e1e2e),
            bold: None,
            ansi: [
                0x45475a, 0xf38ba8, 0xa6e3a1, 0xf9e2af, 0x89b4fa, 0xf5c2e7, 0x94e2d5, 0xbac2de,
                0x585b70, 0xf7aec2, 0xc2ecbf, 0xfcd682, 0xaeccfc, 0xf398da, 0xb1eae1, 0xa6adc8,
            ],
            padding: (25, 20),
            rtl: true,
            minimum_contrast: 1.0,
            opacity: 0.65,
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
