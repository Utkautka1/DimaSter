//! Все настройки со значениями по умолчанию.

use std::path::PathBuf;

use crate::Snippet;
use crate::keys::default_bindings;
use crate::{
    Bell, Binding, CursorLook, Effects, Programs, Smart, Theme, Ui, Widgets, WindowConfig,
};

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

/// Куда попадает выделенное мышью.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CopyOnSelect {
    #[default]
    Primary,
    Clipboard,
    Off,
}

impl CopyOnSelect {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "primary" => Some(Self::Primary),
            "clipboard" => Some(Self::Clipboard),
            "off" => Some(Self::Off),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Clipboard => "clipboard",
            Self::Off => "off",
        }
    }
}

/// Настройки терминала.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// `auto`, `ru`, `en` или код своего файла `lang/<код>.lua`.
    pub language: String,
    /// `None` — моноширинный шрифт из fontconfig.
    pub font_family: Option<String>,
    /// Семейства для жирного, курсива и жирного курсива.
    pub font_styles: [Option<String>; 3],
    // кегль в пунктах (1 pt = 4/3 пикселя при масштабе 1)
    pub font_size: f32,
    /// Дополнительные каталоги со шрифтами.
    pub font_dirs: Vec<PathBuf>,
    pub theme: Theme,
    // отступы от краёв окна в пикселях
    pub padding: (u32, u32),
    pub cursor: CursorStyle,
    pub cursor_look: CursorLook,
    // строк истории
    pub scrollback: usize,
    /// Команда вместо `$SHELL`.
    pub shell: Vec<String>,
    /// Уезжают ли картинки в историю вместе с текстом.
    pub images_scroll: bool,
    /// Переставлять ли RTL-текст при отрисовке.
    pub rtl: bool,
    /// Значение `TERM`.
    pub term: String,
    pub keys: Vec<Binding>,
    pub window: WindowConfig,
    pub effects: Effects,
    /// Восстанавливать окна, папки и историю после перезапуска.
    pub restore_session: bool,
    // сколько строк истории сохранять в сессии
    pub session_history: usize,
    /// Разрешить `dimaster ctl` управлять окнами.
    pub remote_control: bool,
    // множитель высоты строки, 1.0 — как задано шрифтом
    pub line_height: f32,
    pub ligatures: bool,
    /// OpenType-фичи (например `{"-calt", "ss01"}`).
    pub font_features: Vec<String>,
    /// Минимальный контраст текста и фона (1.0 — выкл., до 21.0).
    pub minimum_contrast: f32,
    pub copy_on_select: CopyOnSelect,
    // строк на один щелчок колеса
    pub scroll_multiplier: f32,
    /// Разрешить программам писать в буфер обмена (OSC 52).
    pub clipboard_write: bool,
    /// Предупреждать перед вставкой опасного текста.
    pub clipboard_paste_protection: bool,
    /// Скрывать указатель мыши при наборе текста.
    pub mouse_hide_while_typing: bool,
    /// Спрашивать подтверждение перед закрытием окна.
    pub confirm_close: bool,
    pub smart: Smart,
    /// Свои команды для списка сниппетов.
    pub snippets: Vec<Snippet>,
    /// Полоса прокрутки с метками команд.
    pub scrollbar: bool,
    pub widgets: Widgets,
    pub programs: Programs,
    pub bell: Bell,
    /// Вид интерфейса поверх терминала.
    pub ui: Ui,
    /// Переменные окружения для shell этого терминала.
    pub env: Vec<(String, String)>,
}

impl Default for Config {
    // плоское перечисление значений без логики
    #[allow(clippy::too_many_lines)]
    fn default() -> Self {
        Self {
            language: "auto".to_owned(),
            font_family: None,
            font_styles: Default::default(),
            font_size: 11.0,
            font_dirs: Vec::new(),
            theme: Theme::default(),
            padding: (25, 20),
            cursor: CursorStyle::Bar,
            cursor_look: CursorLook::default(),
            scrollback: 10_000,
            shell: Vec::new(),
            images_scroll: true,
            rtl: true,
            term: "xterm-256color".to_owned(),
            keys: default_bindings(),
            window: WindowConfig::default(),
            effects: Effects::default(),
            restore_session: false,
            session_history: 2000,
            remote_control: true,
            line_height: 1.0,
            ligatures: true,
            font_features: Vec::new(),
            minimum_contrast: 1.0,
            copy_on_select: CopyOnSelect::Primary,
            scroll_multiplier: 3.0,
            clipboard_write: true,
            clipboard_paste_protection: true,
            mouse_hide_while_typing: false,
            confirm_close: false,
            smart: Smart::default(),
            snippets: Vec::new(),
            scrollbar: true,
            widgets: Widgets::default(),
            programs: Programs::default(),
            bell: Bell::default(),
            ui: Ui::default(),
            env: Vec::new(),
        }
    }
}

impl Config {
    /// Новое сочетание заменяет прежние с тем же началом.
    pub fn bind(&mut self, binding: Binding) {
        self.keys.retain(|old| !binding.overrides(old));
        self.keys.push(binding);
    }
}
