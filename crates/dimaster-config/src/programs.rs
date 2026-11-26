//! Таблицы `programs` и `bell`: что разрешено программам и как звенит BEL.

use mlua::Table;

use crate::{Config, ConfigError, parse_color};

/// Что программы могут менять управляющими последовательностями.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Programs {
    pub title: bool,
    pub colors: bool,
    pub cursor_shape: bool,
    pub pointer_shape: bool,
    pub progress: bool,
    pub notifications: bool,
    /// Разрешить программе просить Shift с мышью (XTSHIFTESCAPE).
    pub shift_mouse: bool,
}

impl Default for Programs {
    fn default() -> Self {
        Self {
            title: true,
            colors: true,
            cursor_shape: true,
            pointer_shape: true,
            progress: true,
            notifications: true,
            shift_mouse: true,
        }
    }
}

/// Реакция на звонок.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bell {
    /// Рамка окна вспыхивает.
    pub flash: bool,
    pub color: u32,
    /// Попросить внимания, если окно не в фокусе.
    pub attention: bool,
    pub notify: bool,
    /// Своя программа, например проигрыватель звука; пусто — не запускать.
    pub command: Vec<String>,
}

impl Default for Bell {
    fn default() -> Self {
        Self {
            flash: true,
            color: 0xe0af68,
            attention: true,
            notify: false,
            command: Vec::new(),
        }
    }
}

type Done = Result<(), ConfigError>;

fn flag(table: &Table, key: &str, target: &mut bool) -> Done {
    *target = table.get::<Option<bool>>(key)?.unwrap_or(*target);
    Ok(())
}

fn programs(table: &Table, programs: &mut Programs) -> Done {
    flag(table, "title", &mut programs.title)?;
    flag(table, "colors", &mut programs.colors)?;
    flag(table, "cursor_shape", &mut programs.cursor_shape)?;
    flag(table, "pointer_shape", &mut programs.pointer_shape)?;
    flag(table, "progress", &mut programs.progress)?;
    flag(table, "notifications", &mut programs.notifications)?;
    flag(table, "shift_mouse", &mut programs.shift_mouse)
}

fn bell(table: &Table, bell: &mut Bell) -> Done {
    flag(table, "flash", &mut bell.flash)?;
    flag(table, "attention", &mut bell.attention)?;
    flag(table, "notify", &mut bell.notify)?;
    if let Some(text) = table.get::<Option<String>>("color")? {
        bell.color = parse_color(&text).ok_or_else(|| ConfigError::new("bell.color", &text))?;
    }
    if let Some(command) = table.get::<Option<Vec<String>>>("command")? {
        bell.command = command;
    }
    Ok(())
}

pub(crate) fn apply(table: &Table, config: &mut Config) -> Done {
    if let Some(inner) = table.get::<Option<Table>>("programs")? {
        programs(&inner, &mut config.programs)?;
    }
    match table.get::<Option<Table>>("bell")? {
        Some(inner) => bell(&inner, &mut config.bell),
        None => Ok(()),
    }
}
