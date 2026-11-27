//! Чтение Lua-конфига в [`Config`].

use dimaster_i18n::trf;
use std::fmt;
use std::path::{Path, PathBuf};

use mlua::{Lua, Table, Value};

use crate::import::ghostty_theme;
use crate::theme::builtin_source;
use crate::{Config, CopyOnSelect, Decorations, Effects, Theme, WindowConfig, parse_color};
use crate::{
    context, cursor, help, limits, load_keys, load_smart, load_ui, native, programs, snippets,
    tuning, widgets,
};

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

impl ConfigError {
    pub(crate) fn new(key: &str, value: &str) -> Self {
        invalid(key, value)
    }
}

fn invalid(key: &str, value: impl fmt::Debug) -> ConfigError {
    let value = format!("{value:?}");
    let values = [("key", key), ("value", value.as_str())];
    ConfigError(trf("неверное значение {key}: {value}", &values))
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
    load_with(path, &[])
}

pub fn load_with(path: &Path, overrides: &[String]) -> Result<Config, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(source) => parse_with(&source, path.parent(), overrides),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            parse_with("return {}", None, overrides)
        }
        Err(e) => {
            let (path, error) = (path.display().to_string(), e.to_string());
            let values = [("path", path.as_str()), ("error", error.as_str())];
            Err(ConfigError(trf(
                "не удалось прочитать {path}: {error}",
                &values,
            )))
        }
    }
}

// `~/x` в путях из конфига
pub(crate) fn expand(path: String) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(path),
    }
}

/// `directory` — каталог настроек: оттуда берутся модули и темы.
pub fn parse(source: &str, directory: Option<&Path>) -> Result<Config, ConfigError> {
    parse_with(source, directory, &[])
}

/// То же с наложенными поверх таблицами плагинов (литералы Lua).
pub fn parse_with(
    source: &str,
    directory: Option<&Path>,
    overrides: &[String],
) -> Result<Config, ConfigError> {
    let lua = Lua::new();
    if let Some(directory) = directory {
        context::install(&lua, directory)?;
    }
    let table: Table = lua.load(source).set_name("init.lua").eval()?;
    for literal in overrides {
        let extra: Table = lua.load(format!("return {literal}")).eval()?;
        merge(&table, extra)?;
    }
    from_table(&lua, &table, directory)
}

// словари объединяются по ключам, списки заменяются, `keys` накапливаются
fn merge(target: &Table, extra: Table) -> mlua::Result<()> {
    for pair in extra.pairs::<Value, Value>() {
        let (key, value) = pair?;
        let appended = matches!(&key, Value::String(k) if k.as_bytes() == b"keys");
        match (target.get::<Value>(key.clone())?, value) {
            (Value::Table(old), Value::Table(new)) if appended => {
                for item in new.sequence_values::<Value>() {
                    old.push(item?)?;
                }
            }
            (Value::Table(old), Value::Table(new)) if new.raw_len() == 0 => merge(&old, new)?,
            (_, value) => target.set(key, value)?,
        }
    }
    Ok(())
}

fn from_table(lua: &Lua, table: &Table, directory: Option<&Path>) -> Result<Config, ConfigError> {
    let mut config = Config::default();
    apply_limits(table, &mut config)?;
    if let Some(name) = table.get::<Option<String>>("theme")? {
        load_theme(lua, &name, directory, &mut config.theme)?;
    }
    if let Some(colors) = table.get::<Option<Table>>("colors")? {
        apply_colors(&colors, &mut config.theme)?;
    }
    apply_language(table, &mut config)?;
    apply_font(table, &mut config)?;
    apply_general(table, &mut config)?;
    load_keys::apply(table, &mut config)?;
    programs::apply(table, &mut config)?;
    snippets::apply(table, &mut config)?;
    apply_sections(table, &mut config)?;
    Ok(config)
}

// раньше всего остального: пределы действуют уже при разборе этой же таблицы
fn apply_limits(table: &Table, config: &mut Config) -> Result<(), ConfigError> {
    if let Some(chosen) = table.get::<Option<Table>>("limits")? {
        limits::apply(&chosen, &mut config.limits)?;
        dimaster_limits::apply(&config.limits);
    }
    Ok(())
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
    if let Some(source) = custom.or_else(|| builtin_source(name).map(str::to_owned)) {
        return apply_colors(&lua.load(source).eval()?, theme);
    }
    // настройки DimaSter и Ghostty лежат в одном каталоге ~/.config
    let config_home = directory.and_then(Path::parent);
    *theme = ghostty_theme(name, config_home).ok_or_else(|| invalid("theme", name))?;
    Ok(())
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
    for (slot, key) in config
        .font_styles
        .iter_mut()
        .zip(["bold", "italic", "bold_italic"])
    {
        *slot = font.get(key)?;
    }
    if let Some(size) = font.get::<Option<f32>>("size")? {
        if !(4.0..=200.0).contains(&size) {
            return Err(invalid("font.size", size));
        }
        config.font_size = size;
    }
    ranged(&font, "line_height", 0.8..=2.0, &mut config.line_height)?;
    flag(&font, "ligatures", &mut config.ligatures)?;
    if let Some(features) = font.get::<Option<Vec<String>>>("features")? {
        config.font_features = features;
    }
    let dirs: Option<Vec<String>> = font.get("dirs")?;
    config.font_dirs = dirs.unwrap_or_default().into_iter().map(expand).collect();
    Ok(())
}

fn apply_language(table: &Table, config: &mut Config) -> Result<(), ConfigError> {
    let Some(code) = table.get::<Option<String>>("language")? else {
        return Ok(());
    };
    // код попадает в имя файла
    let safe = |c: char| c.is_ascii_alphanumeric() || "_-".contains(c);
    if code.is_empty() || !code.chars().all(safe) {
        return Err(invalid("language", &code));
    }
    config.language = code;
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
    cursor::apply(&table.get("cursor")?, config)?;
    if let Some(lines) = table.get::<Option<usize>>("scrollback")? {
        // верхняя граница защищает память от опечатки
        config.scrollback = lines.min(dimaster_limits::SCROLLBACK_MAX.size());
    }
    config.shell = table
        .get::<Option<Vec<String>>>("shell")?
        .unwrap_or_default();
    let scroll = table.get::<Option<bool>>("images_scroll")?;
    config.images_scroll = scroll.unwrap_or(config.images_scroll);
    config.rtl = table.get::<Option<bool>>("rtl")?.unwrap_or(config.rtl);
    if let Some(term) = table.get::<Option<String>>("term")? {
        config.term = term;
    }
    ranged(
        table,
        "minimum_contrast",
        1.0..=21.0,
        &mut config.minimum_contrast,
    )?;
    flag(table, "confirm_close", &mut config.confirm_close)?;
    Ok(())
}

fn fraction(table: &Table, key: &str, target: &mut f32) -> Result<(), ConfigError> {
    if let Some(value) = table.get::<Option<f32>>(key)? {
        let (low, high) = dimaster_limits::opacity_range();
        if !(low..=high).contains(&value) {
            return Err(invalid(key, value));
        }
        *target = value;
    }
    Ok(())
}

fn flag(table: &Table, key: &str, target: &mut bool) -> Result<(), ConfigError> {
    *target = table.get::<Option<bool>>(key)?.unwrap_or(*target);
    Ok(())
}

fn apply_window(table: &Table, window: &mut WindowConfig) -> Result<(), ConfigError> {
    if let Some(name) = table.get::<Option<String>>("decorations")? {
        window.decorations =
            Decorations::parse(&name).ok_or_else(|| invalid("decorations", &name))?;
    }
    fraction(table, "opacity", &mut window.opacity)?;
    fraction(table, "background_opacity", &mut window.background_opacity)?;
    flag(table, "blur", &mut window.blur)?;
    let side =
        |value: Option<u32>| value.map(|v| v.clamp(10, dimaster_limits::WINDOW_CELLS_MAX.u32()));
    window.columns = side(table.get("columns")?);
    window.rows = side(table.get("rows")?);
    window.background_dir = table.get::<Option<String>>("background_dir")?.map(expand);
    window.background_image = table.get::<Option<String>>("background_image")?.map(expand);
    Ok(())
}

fn apply_effects(table: &Table, effects: &mut Effects) -> Result<(), ConfigError> {
    flag(table, "cursor_blink", &mut effects.cursor_blink)?;
    flag(table, "cursor_animation", &mut effects.cursor_animation)?;
    flag(table, "cursor_trail", &mut effects.cursor_trail)?;
    flag(table, "smooth_scroll", &mut effects.smooth_scroll)
}

// виджеты и вид интерфейса
fn apply_interface(table: &Table, config: &mut Config) -> Result<(), ConfigError> {
    if let Some(table) = table.get::<Option<Table>>("widgets")? {
        widgets::apply(&table, &mut config.widgets)?;
    }
    if let Some(table) = table.get::<Option<Table>>("ui")? {
        load_ui::apply(&table, &mut config.ui)?;
    }
    Ok(())
}

// вложенные таблицы window и effects, управление и сессии
// приветствие с советами и тонкая настройка
fn apply_extras(table: &Table, config: &mut Config) -> Result<(), ConfigError> {
    if let Some(help_table) = table.get::<Option<Table>>("help")? {
        help::apply(&help_table, &mut config.help)?;
    }
    if let Some(tuning_table) = table.get::<Option<Table>>("tuning")? {
        tuning::apply(&tuning_table, &mut config.tuning)?;
    }
    if let Some(native_table) = table.get::<Option<Table>>("native")? {
        native::apply(&native_table, &mut config.native)?;
    }
    Ok(())
}

fn apply_sections(table: &Table, config: &mut Config) -> Result<(), ConfigError> {
    if let Some(window) = table.get::<Option<Table>>("window")? {
        apply_window(&window, &mut config.window)?;
    }
    if let Some(effects) = table.get::<Option<Table>>("effects")? {
        apply_effects(&effects, &mut config.effects)?;
    }
    apply_interaction(table, config)?;
    if let Some(smart) = table.get::<Option<Table>>("smart")? {
        load_smart::apply(&smart, &mut config.smart)?;
    }
    apply_extras(table, config)?;
    flag(table, "scrollbar", &mut config.scrollbar)?;
    apply_env(table, config)?;
    apply_interface(table, config)?;
    flag(table, "remote_control", &mut config.remote_control)?;
    if let Some(session) = table.get::<Option<Table>>("session")? {
        flag(&session, "restore", &mut config.restore_session)?;
        if let Some(lines) = session.get::<Option<usize>>("history_lines")? {
            config.session_history = lines.min(dimaster_limits::SESSION_LINES_MAX.size());
        }
    }
    Ok(())
}

fn ranged(
    table: &Table,
    key: &str,
    range: std::ops::RangeInclusive<f32>,
    target: &mut f32,
) -> Result<(), ConfigError> {
    if let Some(value) = table.get::<Option<f32>>(key)? {
        if !range.contains(&value) {
            return Err(invalid(key, value));
        }
        *target = value;
    }
    Ok(())
}

// мышь и буфер обмена
fn apply_interaction(table: &Table, config: &mut Config) -> Result<(), ConfigError> {
    if let Some(name) = table.get::<Option<String>>("copy_on_select")? {
        config.copy_on_select =
            CopyOnSelect::parse(&name).ok_or_else(|| invalid("copy_on_select", &name))?;
    }
    ranged(
        table,
        "scroll_multiplier",
        0.1..=50.0,
        &mut config.scroll_multiplier,
    )?;
    flag(table, "clipboard_write", &mut config.clipboard_write)?;
    flag(
        table,
        "clipboard_paste_protection",
        &mut config.clipboard_paste_protection,
    )?;
    flag(
        table,
        "mouse_hide_while_typing",
        &mut config.mouse_hide_while_typing,
    )
}

// значения с `~/` раскрываются: так указывают пути к другим конфигам
fn apply_env(table: &Table, config: &mut Config) -> Result<(), ConfigError> {
    let Some(variables) = table.get::<Option<Table>>("env")? else {
        return Ok(());
    };
    for pair in variables.pairs::<String, String>() {
        let (name, value) = pair?;
        if name.is_empty() || name.contains('=') {
            return Err(invalid("env", &name));
        }
        config
            .env
            .push((name, expand(value).to_string_lossy().into_owned()));
    }
    config.env.sort();
    Ok(())
}
