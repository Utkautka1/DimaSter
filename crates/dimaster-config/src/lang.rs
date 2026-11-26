//! Переводы пользователя: `lang/<код>.lua` в каталоге настроек.

use std::path::Path;

use mlua::{Lua, Table};

use crate::ConfigError;

/// Пары «русская строка — перевод»; нет файла — пустой список.
pub fn translations(directory: &Path, code: &str) -> Result<Vec<(String, String)>, ConfigError> {
    let path = directory.join("lang").join(format!("{code}.lua"));
    let Ok(source) = std::fs::read_to_string(path) else {
        return Ok(Vec::new());
    };
    let table: Table = Lua::new().load(source).set_name("lang.lua").eval()?;
    let pairs = table.pairs::<String, String>();
    Ok(pairs.collect::<mlua::Result<_>>()?)
}
