//! Чтение Lua-конфига в [`Config`].

use std::fmt;
use std::path::{Path, PathBuf};

use mlua::{Lua, Table, Value};

use crate::theme::builtin_source;
use crate::{Action, Binding, Config, CursorStyle, Mods, Theme, parse_color};

/// Понятное человеку описание того, что не так в конфиге.
#[derive(Debug)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

impl From<mlua::Error> for ConfigError {
    fn from(error: mlua::Error) -> Self {
        Self(error.to_string())
    }
}

fn invalid(key: &str, value: impl fmt::Debug) -> ConfigError {
    ConfigError(format!("неверное значение {key}: {value:?}"))
}

/// `$XDG_CONFIG_HOME/dimaster` или `~/.config/dimaster`.
pub fn config_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".config")))?;
    Some(base.join("dimaster"))
}

/// Читает файл; отсутствие файла — не ошибка, а настройки по умолчанию.
pub fn load(path: &Path) -> Result<Config, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(source) => parse(&source, path.parent()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(ConfigError(format!(
            "не удалось прочитать {}: {e}",
            path.display()
        ))),
    }
}

/// `directory` нужен для поиска пользовательских тем.
pub fn parse(source: &str, directory: Option<&Path>) -> Result<Config, ConfigError> {
    let lua = Lua::new();
    let table: Table = lua.load(source).set_name("init.lua").eval()?;
    let mut config = Config::default();
    if let Some(name) = table.get::<Option<String>>("theme")? {
        load_theme(&lua, &name, directory, &mut config.theme)?;
    }
    if let Some(colors) = table.get::<Option<Table>>("colors")? {
        apply_colors(&colors, &mut config.theme)?;
    }
    apply_font(&table, &mut config)?;
    apply_general(&table, &mut config)?;
    apply_keys(&table, &mut config)?;
    Ok(config)
}

// своя тема из themes/<имя>.lua важнее встроенной с тем же именем
fn load_theme(
    lua: &Lua,
    name: &str,
    directory: Option<&Path>,
    theme: &mut Theme,
) -> Result<(), ConfigError> {
    let custom = directory
        .filter(|_| !name.contains(['/', '\\']))
        .and_then(|d| std::fs::read_to_string(d.join("themes").join(format!("{name}.lua"))).ok());
    let source = custom
        .or_else(|| builtin_source(name).map(str::to_owned))
        .ok_or_else(|| invalid("theme", name))?;
    apply_colors(&lua.load(source).eval()?, theme)
}

fn color(table: &Table, key: &str, target: &mut u32) -> Result<(), ConfigError> {
    if let Some(text) = table.get::<Option<String>>(key)? {
        *target = parse_color(&text).ok_or_else(|| invalid(key, &text))?;
    }
    Ok(())
}

fn apply_colors(table: &Table, theme: &mut Theme) -> Result<(), ConfigError> {
    color(table, "foreground", &mut theme.foreground)?;
    color(table, "background", &mut theme.background)?;
    color(table, "cursor", &mut theme.cursor)?;
    color(table, "selection", &mut theme.selection)?;
    if let Some(text) = table.get::<Option<String>>("selection_text")? {
        let parsed = parse_color(&text).ok_or_else(|| invalid("selection_text", &text))?;
        theme.selection_text = Some(parsed);
    }
    let Some(ansi) = table.get::<Option<Table>>("ansi")? else {
        return Ok(());
    };
    for (index, slot) in theme.ansi.iter_mut().enumerate() {
        if let Some(text) = ansi.get::<Option<String>>(index + 1)? {
            *slot = parse_color(&text).ok_or_else(|| invalid("ansi", &text))?;
        }
    }
    Ok(())
}

fn apply_font(table: &Table, config: &mut Config) -> Result<(), ConfigError> {
    let Some(font) = table.get::<Option<Table>>("font")? else {
        return Ok(());
    };
    config.font_family = font.get("family")?;
    if let Some(size) = font.get::<Option<f32>>("size")? {
        if !(4.0..=200.0).contains(&size) {
            return Err(invalid("font.size", size));
        }
        config.font_size = size;
    }
    let dirs: Option<Vec<String>> = font.get("dirs")?;
    config.font_dirs = dirs
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect();
    Ok(())
}

// число — одинаковый отступ, таблица { x, y } — раздельные
fn padding(value: Value) -> Result<(u32, u32), ConfigError> {
    let side = |v: i64| u32::try_from(v).ok().filter(|&v| v <= 500);
    let pair = match &value {
        Value::Integer(v) => side(*v).map(|v| (v, v)),
        Value::Table(t) => side(t.get("x")?).zip(side(t.get("y")?)),
        _ => None,
    };
    pair.ok_or_else(|| invalid("padding", value))
}

fn apply_general(table: &Table, config: &mut Config) -> Result<(), ConfigError> {
    let value: Value = table.get("padding")?;
    if !value.is_nil() {
        config.padding = padding(value)?;
    }
    if let Some(name) = table.get::<Option<String>>("cursor")? {
        config.cursor = CursorStyle::parse(&name).ok_or_else(|| invalid("cursor", &name))?;
    }
    if let Some(lines) = table.get::<Option<usize>>("scrollback")? {
        // верхняя граница защищает память от опечатки
        config.scrollback = lines.min(1_000_000);
    }
    config.shell = table
        .get::<Option<Vec<String>>>("shell")?
        .unwrap_or_default();
    config.rtl = table.get::<Option<bool>>("rtl")?.unwrap_or(config.rtl);
    if let Some(term) = table.get::<Option<String>>("term")? {
        config.term = term;
    }
    Ok(())
}

fn apply_keys(table: &Table, config: &mut Config) -> Result<(), ConfigError> {
    let Some(keys) = table.get::<Option<Vec<Table>>>("keys")? else {
        return Ok(());
    };
    for entry in keys {
        let key: String = entry.get("key")?;
        let mods: String = entry.get::<Option<String>>("mods")?.unwrap_or_default();
        let action: String = entry.get("action")?;
        config.bind(Binding {
            key: key.to_lowercase(),
            mods: Mods::parse(&mods).ok_or_else(|| invalid("mods", &mods))?,
            action: Action::parse(&action).ok_or_else(|| invalid("action", &action))?,
        });
    }
    Ok(())
}
