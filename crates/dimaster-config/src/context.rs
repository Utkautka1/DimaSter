//! Контекст запуска, который Lua видит в глобальной таблице `dimaster`.

use std::path::Path;

use dimaster_i18n::tr;

use mlua::{Lua, Table};

// имя становится частью пути модуля, поэтому допускаются только простые символы
fn slug(text: &str) -> String {
    text.trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .collect()
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

fn build(lua: &Lua, directory: &Path) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
    table.set("hostname", slug(&hostname))?;
    // "ubuntu:GNOME" -> "gnome"
    let desktop = env("XDG_CURRENT_DESKTOP");
    table.set(
        "desktop",
        slug(desktop.rsplit(':').next().unwrap_or_default()),
    )?;
    table.set("session", slug(&env("XDG_SESSION_TYPE")))?;
    table.set("ssh", !env("SSH_CONNECTION").is_empty())?;
    table.set("profile", slug(&env("DIMASTER_PROFILE")))?;
    table.set("config_dir", directory.to_string_lossy().into_owned())?;
    let translate = lua.create_function(|_, text: String| Ok(tr(&text)))?;
    table.set("tr", translate)?;
    Ok(table)
}

/// Открывает конфигу `require` из его каталога и сообщает контекст.
pub(crate) fn install(lua: &Lua, directory: &Path) -> mlua::Result<()> {
    let package: Table = lua.globals().get("package")?;
    let root = directory.to_string_lossy();
    // только каталог настроек: системные модули Lua здесь не нужны
    package.set("path", format!("{root}/?.lua;{root}/?/init.lua"))?;
    lua.globals().set("dimaster", build(lua, directory)?)
}
