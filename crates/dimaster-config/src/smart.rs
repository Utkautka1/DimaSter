//! Настройки «умной оболочки»: подсказки под командами и реакция на долгие.

/// Таблица `smart` из конфига.
#[derive(Debug, Clone, PartialEq)]
pub struct Smart {
    /// Подключать ли скрипты интеграции к zsh, bash и fish.
    pub integration: bool,
    /// Своё «команда не найдена» на языке терминала.
    pub not_found: bool,
    // пустая строка отключает подсказку
    pub ok: String,
    pub empty: String,
    pub errors: bool,
    pub search_tools: Vec<String>,
    // секунды; 0 — не реагировать на долгие команды
    pub long_command: f32,
    pub glow: bool,
    pub glow_color: u32,
    pub notify: bool,
    /// Двойной Alt вызывает Sudo Fix.
    pub double_alt_sudo: bool,
}

impl Default for Smart {
    fn default() -> Self {
        Self {
            integration: true,
            not_found: true,
            ok: "➜ OK".to_owned(),
            empty: "∅ Ничего не найдено".to_owned(),
            errors: true,
            search_tools: ["grep", "rg", "find", "fd", "ls", "locate"]
                .map(str::to_owned)
                .to_vec(),
            long_command: 10.0,
            glow: true,
            glow_color: 0x7aa2f7,
            notify: true,
            double_alt_sudo: false,
        }
    }
}
