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
    HintMode,
    CommandHistory,
    PickFile,
    JumpDir,
    /// Серверы из `~/.ssh/config`.
    SshHosts,
    /// Свои команды из `snippets`.
    Snippets,
    /// Контейнеры Docker.
    Docker,
    SudoFix,
    Presentation,
    Minimap,
    ToggleMonitor,
    ToggleStatusBar,
    NewWindow,
    /// Прокрутить к предыдущему приглашению shell (нужны метки OSC 133).
    PromptPrevious,
    PromptNext,
    /// Скопировать вывод последней завершившейся команды.
    CopyLastOutput,
    /// Отправить программе заданный текст.
    Send(String),
    /// Команда плагина по имени.
    Plugin(String),
    /// Проглотить нажатие: ни терминал, ни программа его не получают.
    Ignore,
    /// Включить таблицу привязок; `once` — до первой сработавшей.
    ActivateTable {
        name: String,
        once: bool,
    },
    DeactivateTable,
    DeactivateAllTables,
    /// Отправить программе уже нажатое начало цепочки.
    EndSequence,
    /// Отключить сочетание, чтобы оно уходило программе.
    None,
}

const ACTIONS: [(&str, Action); 37] = [
    ("ssh_hosts", Action::SshHosts),
    ("snippets", Action::Snippets),
    ("docker", Action::Docker),
    ("copy_last_output", Action::CopyLastOutput),
    ("ignore", Action::Ignore),
    ("unbind", Action::None),
    ("deactivate_key_table", Action::DeactivateTable),
    ("deactivate_all_key_tables", Action::DeactivateAllTables),
    ("end_key_sequence", Action::EndSequence),
    ("minimap", Action::Minimap),
    ("toggle_monitor", Action::ToggleMonitor),
    ("toggle_statusbar", Action::ToggleStatusBar),
    ("hint_mode", Action::HintMode),
    ("command_history", Action::CommandHistory),
    ("pick_file", Action::PickFile),
    ("jump_dir", Action::JumpDir),
    ("sudo_fix", Action::SudoFix),
    ("presentation", Action::Presentation),
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

    // действия вида `имя:параметр`
    fn with_parameter(name: &str, value: &str) -> Option<Self> {
        let value_owned = value.to_owned();
        Some(match name {
            "send" => Self::Send(value_owned),
            "text" => Self::Send(unescape(value)?),
            "csi" => Self::Send(format!("\x1b[{value}")),
            "esc" => Self::Send(format!("\x1b{value}")),
            "plugin" => Self::Plugin(value_owned),
            "activate_key_table" => Self::ActivateTable {
                name: value_owned,
                once: false,
            },
            "activate_key_table_once" => Self::ActivateTable {
                name: value_owned,
                once: true,
            },
            _ => return None,
        })
    }

    pub fn parse(name: &str) -> Option<Self> {
        if let Some((action, value)) = name.split_once(':') {
            return Self::with_parameter(action, value);
        }
        ACTIONS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, action)| action.clone())
    }
}

// экранирование строк Zig, как в `text:` у Ghostty: \x15, \n, \e, \\
fn unescape(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        out.push(match chars.next()? {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'e' => '\x1b',
            'x' => hex_byte(chars.next()?, chars.next()?)?,
            other => other,
        });
    }
    Some(out)
}

// только ASCII: старший байт в строке стал бы двумя байтами UTF-8
fn hex_byte(high: char, low: char) -> Option<char> {
    let value = high.to_digit(16)? * 16 + low.to_digit(16)?;
    char::from_u32(value).filter(char::is_ascii)
}

/// Одно нажатие: строчный символ или имя (`pageup`, `f5`, `keya`,
/// `catch_all`) с модификаторами.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    pub key: String,
    pub mods: Mods,
}

// `key_a` и `KeyA` — одна и та же физическая клавиша
fn normalized(key: &str) -> String {
    let key = key.trim().to_lowercase();
    match key.strip_prefix("key_").map(|k| format!("key{k}")) {
        Some(physical) => physical,
        None => key.replace("digit_", "digit"),
    }
}

impl Chord {
    pub fn new(key: &str, mods: Mods) -> Self {
        Self {
            key: normalized(key),
            mods,
        }
    }

    /// `ctrl+shift+a`; сам знак `+` пишется последним: `ctrl++`.
    pub fn parse(text: &str) -> Option<Self> {
        let (mods, key) = match text.strip_suffix('+') {
            Some(rest) => (rest.trim_end_matches('+'), "+"),
            None => text.rsplit_once('+').unwrap_or(("", text)),
        };
        (!key.trim().is_empty()).then_some(())?;
        Some(Self::new(key, Mods::parse(mods)?))
    }
}

/// Привязка: цепочка нажатий и действия по порядку.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub chords: Vec<Chord>,
    pub actions: Vec<Action>,
    /// Нажатие после действий уходит и программе.
    pub unconsumed: bool,
    /// Забирать клавишу, только если действие сейчас возможно.
    pub performable: bool,
    /// Имя таблицы; пустое — основная.
    pub table: String,
}

impl Binding {
    pub fn new(chords: Vec<Chord>, actions: Vec<Action>) -> Self {
        Self {
            chords,
            actions,
            unconsumed: false,
            performable: false,
            table: String::new(),
        }
    }

    /// Новая привязка вытесняет прежнюю с тем же началом: `ctrl+a` отменяет
    /// все `ctrl+a>…` и наоборот.
    pub fn overrides(&self, old: &Binding) -> bool {
        let shared = self.chords.len().min(old.chords.len());
        self.table == old.table && self.chords[..shared] == old.chords[..shared]
    }
}

// сочетания по умолчанию совпадают с привычными по Ghostty и kitty
const DEFAULTS: [(&str, &str, Action); 32] = [
    ("e", "ctrl+shift", Action::SshHosts),
    ("i", "ctrl+shift", Action::Snippets),
    ("d", "ctrl+shift", Action::Docker),
    ("x", "ctrl+shift", Action::CopyLastOutput),
    ("m", "ctrl+shift", Action::Minimap),
    ("o", "ctrl+shift", Action::ToggleMonitor),
    ("b", "ctrl+shift", Action::ToggleStatusBar),
    ("u", "ctrl+shift", Action::HintMode),
    ("h", "ctrl+shift", Action::CommandHistory),
    ("t", "ctrl+shift", Action::PickFile),
    ("g", "ctrl+shift", Action::JumpDir),
    ("s", "ctrl+shift", Action::SudoFix),
    ("z", "ctrl+shift", Action::Presentation),
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
            let chord = Chord::new(key, Mods::parse(mods)?);
            Some(Binding::new(vec![chord], vec![action.clone()]))
        })
        .collect()
}
