//! Сочетания клавиш и действия терминала.

/// Модификаторы сочетания.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub logo: bool,
}

impl Mods {
    /// Разбирает строку вида `ctrl+shift`.
    pub fn parse(text: &str) -> Option<Self> {
        let mut mods = Self::default();
        for part in text.split('+').filter(|p| !p.is_empty()) {
            match part.trim().to_ascii_lowercase().as_str() {
                "ctrl" | "control" => mods.ctrl = true,
                "shift" => mods.shift = true,
                "alt" => mods.alt = true,
                "super" | "logo" => mods.logo = true,
                _ => return None,
            }
        }
        Some(mods)
    }
}

/// Действие, которое выполняет сам терминал.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Copy,
    Paste,
    PastePrimary,
    ScrollPageUp,
    ScrollPageDown,
    ScrollTop,
    ScrollBottom,
    FontIncrease,
    FontDecrease,
    FontReset,
    ReloadConfig,
    Search,
    ViMode,
    CommandPalette,
    ToggleFullscreen,
    NewWindow,
    /// Прокрутить к предыдущему приглашению shell (нужны метки OSC 133).
    PromptPrevious,
    PromptNext,
    /// Отправить программе заданный текст.
    Send(String),
    /// Команда плагина по имени.
    Plugin(String),
    /// Отключить сочетание, чтобы оно уходило программе.
    None,
}

const ACTIONS: [(&str, Action); 19] = [
    ("toggle_fullscreen", Action::ToggleFullscreen),
    ("new_window", Action::NewWindow),
    ("prompt_previous", Action::PromptPrevious),
    ("prompt_next", Action::PromptNext),
    ("search", Action::Search),
    ("vi_mode", Action::ViMode),
    ("command_palette", Action::CommandPalette),
    ("copy", Action::Copy),
    ("paste", Action::Paste),
    ("paste_primary", Action::PastePrimary),
    ("scroll_page_up", Action::ScrollPageUp),
    ("scroll_page_down", Action::ScrollPageDown),
    ("scroll_top", Action::ScrollTop),
    ("scroll_bottom", Action::ScrollBottom),
    ("font_increase", Action::FontIncrease),
    ("font_decrease", Action::FontDecrease),
    ("font_reset", Action::FontReset),
    ("reload_config", Action::ReloadConfig),
    ("none", Action::None),
];

impl Action {
    /// Имена действий без параметров — для документации и подсказок.
    pub fn names() -> impl Iterator<Item = &'static str> {
        ACTIONS.iter().map(|(name, _)| *name)
    }

    pub fn parse(name: &str) -> Option<Self> {
        if let Some(text) = name.strip_prefix("send:") {
            return Some(Self::Send(text.to_owned()));
        }
        if let Some(command) = name.strip_prefix("plugin:") {
            return Some(Self::Plugin(command.to_owned()));
        }
        ACTIONS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, action)| action.clone())
    }
}

/// Одно сочетание. `key` — строчный символ или имя (`pageup`, `f5`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub key: String,
    pub mods: Mods,
    pub action: Action,
}

// сочетания по умолчанию совпадают с привычными по Ghostty и kitty
const DEFAULTS: [(&str, &str, Action); 19] = [
    ("enter", "ctrl", Action::ToggleFullscreen),
    ("n", "ctrl+shift", Action::NewWindow),
    ("arrowup", "ctrl+shift", Action::PromptPrevious),
    ("arrowdown", "ctrl+shift", Action::PromptNext),
    ("f", "ctrl+shift", Action::Search),
    ("space", "ctrl+shift", Action::ViMode),
    ("p", "ctrl+shift", Action::CommandPalette),
    ("c", "ctrl+shift", Action::Copy),
    ("v", "ctrl+shift", Action::Paste),
    ("insert", "shift", Action::PastePrimary),
    ("pageup", "shift", Action::ScrollPageUp),
    ("pagedown", "shift", Action::ScrollPageDown),
    ("home", "shift", Action::ScrollTop),
    ("end", "shift", Action::ScrollBottom),
    ("=", "ctrl", Action::FontIncrease),
    ("=", "ctrl+shift", Action::FontIncrease),
    ("-", "ctrl", Action::FontDecrease),
    ("0", "ctrl", Action::FontReset),
    (",", "ctrl+shift", Action::ReloadConfig),
];

pub(crate) fn default_bindings() -> Vec<Binding> {
    DEFAULTS
        .iter()
        .filter_map(|(key, mods, action)| {
            Some(Binding {
                key: (*key).to_owned(),
                mods: Mods::parse(mods)?,
                action: action.clone(),
            })
        })
        .collect()
}
