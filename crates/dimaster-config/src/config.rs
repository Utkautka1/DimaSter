//! Все настройки со значениями по умолчанию.

use std::path::PathBuf;

use crate::keys::default_bindings;
use crate::{Binding, Theme};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    #[default]
    Block,
    Underline,
    Bar,
}

impl CursorStyle {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "block" | "beam_block" => Some(Self::Block),
            "underline" => Some(Self::Underline),
            "bar" | "beam" => Some(Self::Bar),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Underline => "underline",
            Self::Bar => "bar",
        }
    }
}

/// Настройки терминала.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// `None` — моноширинный шрифт из fontconfig.
    pub font_family: Option<String>,
    // кегль в пунктах (1 pt = 4/3 пикселя при масштабе 1)
    pub font_size: f32,
    /// Дополнительные каталоги со шрифтами.
    pub font_dirs: Vec<PathBuf>,
    pub theme: Theme,
    // отступы от краёв окна в пикселях
    pub padding: (u32, u32),
    pub cursor: CursorStyle,
    // строк истории
    pub scrollback: usize,
    /// Команда вместо `$SHELL`.
    pub shell: Vec<String>,
    /// Переставлять ли RTL-текст при отрисовке.
    pub rtl: bool,
    /// Значение `TERM`.
    pub term: String,
    pub keys: Vec<Binding>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font_family: None,
            font_size: 12.0,
            font_dirs: Vec::new(),
            theme: Theme::default(),
            padding: (6, 6),
            cursor: CursorStyle::Block,
            scrollback: 10_000,
            shell: Vec::new(),
            rtl: true,
            term: "xterm-256color".to_owned(),
            keys: default_bindings(),
        }
    }
}

impl Config {
    /// Новое сочетание заменяет старое с теми же клавишами.
    pub fn bind(&mut self, binding: Binding) {
        self.keys
            .retain(|b| (&b.key, b.mods) != (&binding.key, binding.mods));
        self.keys.push(binding);
    }
}
