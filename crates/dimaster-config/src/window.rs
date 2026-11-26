//! Настройки окна и анимаций.

use std::path::PathBuf;

/// Чья рамка у окна.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Decorations {
    /// Своя полоса заголовка DimaSter.
    #[default]
    Own,
    System,
    None,
}

impl Decorations {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "own" => Some(Self::Own),
            "system" => Some(Self::System),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Own => "own",
            Self::System => "system",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowConfig {
    pub decorations: Decorations,
    // непрозрачность фона: 0.0 — стекло, 1.0 — сплошной
    pub opacity: f32,
    /// Просить композитор размыть то, что под окном.
    pub blur: bool,
    pub background_image: Option<PathBuf>,
    // насколько заметна фоновая картинка, 0.0..=1.0
    pub background_opacity: f32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            decorations: Decorations::Own,
            opacity: 1.0,
            blur: false,
            background_image: None,
            background_opacity: 0.3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effects {
    pub cursor_blink: bool,
    /// Плавное перемещение курсора.
    pub cursor_animation: bool,
    /// «Хвост» за курсором при больших прыжках.
    pub cursor_trail: bool,
    pub smooth_scroll: bool,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            cursor_blink: false,
            cursor_animation: true,
            cursor_trail: false,
            smooth_scroll: true,
        }
    }
}
