//! Оформление `fetch` на Lua: Rust передаёт факты, скрипт возвращает макет.

use dimaster_i18n::tr;
use std::path::{Path, PathBuf};

use mlua::{Lua, Table};

use crate::load::expand;
use crate::{ConfigError, context};

/// Встроенный сценарий; им же заполняется пользовательский `fetch.lua`.
pub const DEFAULT_FETCH: &str = include_str!("scaffold/files/fetch.lua");

const COLORS: [(&str, &str); 9] = [
    ("black", "30"),
    ("red", "31"),
    ("green", "32"),
    ("yellow", "33"),
    ("blue", "34"),
    ("magenta", "35"),
    ("cyan", "36"),
    ("white", "37"),
    ("bold", "1"),
];

/// Что напечатать: строки сведений и логотип слева от них.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Layout {
    pub lines: Vec<String>,
    pub logo: Vec<String>,
    /// Картинка вместо текстового логотипа.
    pub image: Option<PathBuf>,
    // ширина колонки логотипа в ячейках
    pub logo_width: usize,
}

// сброс после каждого куска; вложенный color("bold", color(..)) тоже работает
fn paint(lua: &Lua, (name, text): (String, String)) -> mlua::Result<mlua::LuaString> {
    let code = COLORS.iter().find(|c| c.0 == name).map_or("0", |c| c.1);
    lua.create_string(format!("\x1b[{code}m{text}\x1b[0m"))
}

fn prepare(lua: &Lua, facts: &[(&str, Option<String>)]) -> mlua::Result<()> {
    let info = lua.create_table()?;
    for (name, value) in facts {
        if let Some(value) = value {
            info.set(*name, value.as_str())?;
        }
    }
    lua.globals().set("info", info)?;
    let translate = lua.create_function(|_, text: String| Ok(tr(&text)))?;
    lua.globals().set("tr", translate)?;
    lua.globals().set("color", lua.create_function(paint)?)
}

/// Выполняет сценарий; `directory` открывает ему `require` и контекст.
pub fn run_fetch(
    source: &str,
    directory: Option<&Path>,
    facts: &[(&str, Option<String>)],
) -> Result<Layout, ConfigError> {
    let lua = Lua::new();
    if let Some(directory) = directory {
        context::install(&lua, directory)?;
    }
    prepare(&lua, facts)?;
    let table: Table = lua.load(source).set_name("fetch.lua").eval()?;
    let list = |key| {
        table
            .get::<Option<Vec<String>>>(key)
            .map(Option::unwrap_or_default)
    };
    Ok(Layout {
        lines: list("lines")?,
        logo: list("logo")?,
        image: table.get::<Option<String>>("image")?.map(expand),
        logo_width: table
            .get::<Option<usize>>("logo_width")?
            .unwrap_or(22)
            .min(200),
    })
}
