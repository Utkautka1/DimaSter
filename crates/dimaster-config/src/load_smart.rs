//! Разбор таблицы `smart`.

use mlua::Table;

use crate::{ConfigError, Smart, parse_color};

fn set<T: mlua::FromLua>(table: &Table, key: &str, target: &mut T) -> mlua::Result<()> {
    if let Some(value) = table.get::<Option<T>>(key)? {
        *target = value;
    }
    Ok(())
}

fn texts(table: &Table, smart: &mut Smart) -> mlua::Result<()> {
    set(table, "ok", &mut smart.ok)?;
    set(table, "empty", &mut smart.empty)?;
    set(table, "errors", &mut smart.errors)?;
    set(table, "search_tools", &mut smart.search_tools)
}

fn reactions(table: &Table, smart: &mut Smart) -> mlua::Result<()> {
    set(table, "integration", &mut smart.integration)?;
    set(table, "long_command", &mut smart.long_command)?;
    set(table, "glow", &mut smart.glow)?;
    set(table, "notify", &mut smart.notify)?;
    set(table, "double_alt_sudo", &mut smart.double_alt_sudo)
}

pub(crate) fn apply(table: &Table, smart: &mut Smart) -> Result<(), ConfigError> {
    texts(table, smart)?;
    reactions(table, smart)?;
    if let Some(text) = table.get::<Option<String>>("glow_color")? {
        smart.glow_color =
            parse_color(&text).ok_or_else(|| ConfigError::new("glow_color", &text))?;
    }
    smart.long_command = smart.long_command.max(0.0);
    Ok(())
}
