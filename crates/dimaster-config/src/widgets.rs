//! Настройки виджетов: строка статуса, монитор ресурсов, миникарта.

use mlua::Table;

use crate::ConfigError;

/// Таблица `widgets` из конфига.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Widgets {
    pub statusbar: bool,
    // секунды между обновлениями данных
    pub interval: f32,
    /// Считать ли контейнеры Docker.
    pub docker: bool,
    /// Считать изменённые файлы через `git status`. Выключено: репозиторий
    /// может заставить git выполнить свою программу (фильтры в `.git/config`).
    pub git_changes: bool,
    /// Монитор ресурсов в углу окна.
    pub monitor: bool,
    /// Миникарта видна постоянно, а не только в своём режиме.
    pub minimap: bool,
}

impl Default for Widgets {
    fn default() -> Self {
        Self {
            statusbar: true,
            interval: 3.0,
            docker: true,
            git_changes: false,
            monitor: false,
            minimap: false,
        }
    }
}

fn set<T: mlua::FromLua>(table: &Table, key: &str, target: &mut T) -> mlua::Result<()> {
    if let Some(value) = table.get::<Option<T>>(key)? {
        *target = value;
    }
    Ok(())
}

pub(crate) fn apply(table: &Table, widgets: &mut Widgets) -> Result<(), ConfigError> {
    set(table, "statusbar", &mut widgets.statusbar)?;
    set(table, "interval", &mut widgets.interval)?;
    set(table, "docker", &mut widgets.docker)?;
    set(table, "git_changes", &mut widgets.git_changes)?;
    set(table, "monitor", &mut widgets.monitor)?;
    set(table, "minimap", &mut widgets.minimap)?;
    // чаще раза в секунду опрашивать незачем, реже минуты — бессмысленно
    widgets.interval = widgets.interval.clamp(1.0, 60.0);
    Ok(())
}
