//! Настройки `dimaster view` из `view.lua`.
//!
//! Lua остаётся живой, пока открыт просмотрщик: функции колонок и строки
//! состояния вызываются при отрисовке. Дальше этого модуля уходят только
//! готовые строки и цвета.

use std::path::Path;

use mlua::{Function, Lua, Table, Value};

use crate::{ConfigError, config_dir, parse_color};

/// Цвет: номер палитры терминала или точный 0xRRGGBB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewColor {
    Index(u8),
    Rgb(u32),
}

/// Как показывать одну колонку.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewColumn {
    /// Номер колонки в результате запроса.
    pub source: usize,
    pub title: String,
    /// Точная ширина в знакоместах; `None` — по содержимому.
    pub width: Option<usize>,
    pub right: bool,
    pub color: Option<ViewColor>,
}

/// Цвета частей экрана; `None` — начертание по умолчанию.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewColors {
    pub header: Option<ViewColor>,
    pub text: Option<ViewColor>,
    pub selected: Option<ViewColor>,
    pub selected_text: Option<ViewColor>,
    pub status: Option<ViewColor>,
    pub zebra: Option<ViewColor>,
    pub numbers: Option<ViewColor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewSettings {
    pub enabled: bool,
    pub max_width: usize,
    pub gap: usize,
    pub fit: bool,
    pub header: bool,
    pub row_numbers: bool,
    /// Принимать команды `dimaster view --send`.
    pub remote: bool,
    pub colors: ViewColors,
    /// Клавиша и действие; `None` снимает назначение.
    pub keys: Vec<(String, Option<String>)>,
}

impl Default for ViewSettings {
    fn default() -> Self {
        let colors = ViewColors {
            header: Some(ViewColor::Index(4)),
            text: None,
            selected: None,
            selected_text: None,
            status: Some(ViewColor::Index(5)),
            zebra: None,
            numbers: Some(ViewColor::Index(8)),
        };
        Self {
            enabled: true,
            max_width: 40,
            gap: 2,
            fit: false,
            header: true,
            row_numbers: false,
            remote: false,
            colors,
            keys: Vec::new(),
        }
    }
}

/// Загруженный `view.lua` вместе с его функциями.
#[derive(Default)]
pub struct Viewer {
    pub settings: ViewSettings,
    script: Option<(Lua, Table)>,
}

const NAMES: [&str; 8] = [
    "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
];

// "blue", "bright_blue", 0–255 или "#rrggbb"
fn color(value: &Value) -> Option<ViewColor> {
    if let Value::Integer(index) = value {
        return u8::try_from(*index).ok().map(ViewColor::Index);
    }
    let text = value.as_string()?.to_str().ok()?.to_string();
    let (base, name) = match text.strip_prefix("bright_") {
        Some(rest) => (8, rest),
        None => (0, text.as_str()),
    };
    let named = NAMES.iter().position(|n| *n == name);
    match named {
        Some(index) => Some(ViewColor::Index(base + index as u8)),
        None => parse_color(&text).map(ViewColor::Rgb),
    }
}

fn colored(
    table: &Table,
    key: &str,
    default: Option<ViewColor>,
) -> mlua::Result<Option<ViewColor>> {
    match table.get::<Value>(key)? {
        Value::Nil => Ok(default),
        Value::Boolean(false) => Ok(None),
        value => color(&value)
            .map(Some)
            .ok_or_else(|| mlua::Error::runtime(format!("view.colors.{key}"))),
    }
}

fn colors(root: &Table, base: ViewColors) -> mlua::Result<ViewColors> {
    let Some(table) = root.get::<Option<Table>>("colors")? else {
        return Ok(base);
    };
    Ok(ViewColors {
        header: colored(&table, "header", base.header)?,
        text: colored(&table, "text", base.text)?,
        selected: colored(&table, "selected", base.selected)?,
        selected_text: colored(&table, "selected_text", base.selected_text)?,
        status: colored(&table, "status", base.status)?,
        zebra: colored(&table, "zebra", base.zebra)?,
        numbers: colored(&table, "numbers", base.numbers)?,
    })
}

fn keys(root: &Table) -> mlua::Result<Vec<(String, Option<String>)>> {
    let Some(table) = root.get::<Option<Table>>("keys")? else {
        return Ok(Vec::new());
    };
    let pairs = table.pairs::<String, Value>();
    pairs
        .map(|pair| {
            let (key, action) = pair?;
            let action = action.as_string().map(|s| s.to_string_lossy());
            Ok((key, action))
        })
        .collect()
}

fn settings(root: &Table) -> mlua::Result<ViewSettings> {
    let base = ViewSettings::default();
    let flag = |key: &str, default: bool| {
        Ok::<_, mlua::Error>(root.get::<Option<bool>>(key)?.unwrap_or(default))
    };
    let size = |key: &str, default: usize| {
        Ok::<_, mlua::Error>(root.get::<Option<usize>>(key)?.unwrap_or(default))
    };
    Ok(ViewSettings {
        enabled: flag("enabled", base.enabled)?,
        max_width: size("max_width", base.max_width)?.max(1),
        gap: size("gap", base.gap)?.min(16),
        fit: flag("fit", base.fit)?,
        header: flag("header", base.header)?,
        row_numbers: flag("row_numbers", base.row_numbers)?,
        remote: flag("remote", base.remote)?,
        colors: colors(root, base.colors)?,
        keys: keys(root)?,
    })
}

// правила колонок: общие, а поверх них правила именно этой таблицы
fn rule_sets(root: &Table, table: &str) -> Vec<Table> {
    let own = root
        .get::<Table>("tables")
        .and_then(|t| t.get::<Table>(table))
        .and_then(|t| t.get::<Table>("columns"));
    own.into_iter()
        .chain(root.get::<Table>("columns"))
        .collect()
}

// первое заданное значение поля колонки
fn field<T: mlua::FromLua>(sets: &[Table], column: &str, key: &str) -> Option<T> {
    sets.iter().find_map(|set| {
        let rule = set.get::<Table>(column).ok()?;
        rule.get::<Option<T>>(key).ok().flatten()
    })
}

fn listed(sets: &[Table], key: &str) -> Vec<String> {
    let found = sets.iter().find_map(|s| s.get::<Vec<String>>(key).ok());
    found.unwrap_or_default()
}

fn column(sets: &[Table], source: usize, name: &str) -> ViewColumn {
    let align: Option<String> = field(sets, name, "align");
    let paint: Option<Value> = field(sets, name, "color");
    ViewColumn {
        source,
        title: field(sets, name, "title").unwrap_or_else(|| name.to_owned()),
        width: field(sets, name, "width"),
        right: align.as_deref() == Some("right"),
        color: paint.as_ref().and_then(color),
    }
}

// сначала перечисленные в `order`, остальные в порядке запроса
fn ordered(names: &[String], order: &[String], hidden: &[String]) -> Vec<usize> {
    let first = order
        .iter()
        .filter_map(|o| names.iter().position(|n| n == o));
    let mut found: Vec<usize> = first.collect();
    found.extend(
        (0..names.len())
            .filter(|&i| !found.contains(&i))
            .collect::<Vec<_>>(),
    );
    found.retain(|&i| !hidden.contains(&names[i]));
    found
}

fn record(lua: &Lua, row: &[(&str, String)]) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (name, value) in row {
        table.set(*name, value.as_str())?;
    }
    Ok(table)
}

impl Viewer {
    /// `view.lua` из каталога настроек; без файла — значения по умолчанию.
    pub fn load() -> Result<Self, ConfigError> {
        let path = config_dir().map(|d| d.join("view.lua"));
        match path.filter(|p| p.is_file()) {
            Some(path) => Self::read(&path),
            None => Ok(Self::default()),
        }
    }

    fn read(path: &Path) -> Result<Self, ConfigError> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::new("view.lua", &e.to_string()))?;
        Self::parse(&source)
    }

    /// Разбирает текст `view.lua`.
    pub fn parse(source: &str) -> Result<Self, ConfigError> {
        let lua = Lua::new();
        let root: Table = lua.load(source).set_name("view.lua").eval()?;
        Ok(Self {
            settings: settings(&root)?,
            script: Some((lua, root)),
        })
    }

    /// Видимые колонки таблицы по порядку.
    pub fn columns(&self, table: &str, names: &[String]) -> Vec<ViewColumn> {
        let sets = self.script.as_ref().map(|s| rule_sets(&s.1, table));
        let sets = sets.unwrap_or_default();
        let found = ordered(names, &listed(&sets, "order"), &listed(&sets, "hidden"));
        let built = found.into_iter().map(|i| column(&sets, i, &names[i]));
        built.collect()
    }

    /// Ответ функции `format` колонки: текст и, возможно, цвет.
    pub fn cell(
        &self,
        table: &str,
        name: &str,
        value: &str,
        row: &[(&str, String)],
    ) -> Option<Result<(String, Option<ViewColor>), String>> {
        let (lua, root) = self.script.as_ref()?;
        let format: Function = field(&rule_sets(root, table), name, "format")?;
        let called =
            record(lua, row).and_then(|row| format.call::<(Option<String>, Value)>((value, row)));
        Some(match called {
            Ok((text, paint)) => Ok((text.unwrap_or_else(|| value.to_owned()), color(&paint))),
            Err(e) => Err(e.to_string()),
        })
    }

    /// Своя строка состояния из функции `status`.
    pub fn status(&self, info: &[(&str, String)]) -> Option<Result<String, String>> {
        let (lua, root) = self.script.as_ref()?;
        let format = root.get::<Function>("status").ok()?;
        let called = record(lua, info).and_then(|i| format.call::<String>(i));
        Some(called.map_err(|e| e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCRIPT: &str = r##"return {
      gap = 1, colors = { header = "#ff0000", status = false },
      keys = { x = "quit", j = false },
      columns = { order = { "c" }, hidden = { "b" }, a = { title = "A", width = 5 } },
      tables = { t = { columns = { a = { align = "right",
        format = function(v, row) return v .. row.c, "bright_red" end } } } },
      status = function(info) return "row " .. info.row end,
    }"##;

    #[test]
    fn script_shapes_columns_cells_and_status() {
        let viewer = Viewer::parse(SCRIPT).expect("view.lua");
        assert_eq!(viewer.settings.gap, 1);
        assert_eq!(viewer.settings.colors.status, None);
        let names = ["a", "b", "c"].map(String::from);
        let columns = viewer.columns("t", &names);
        assert_eq!(columns.iter().map(|c| c.source).collect::<Vec<_>>(), [2, 0]);
        assert_eq!((columns[1].title.as_str(), columns[1].right), ("A", true));
        let row = [("c", "!".to_owned())];
        let cell = viewer.cell("t", "a", "1", &row).expect("функция");
        assert_eq!(cell, Ok(("1!".into(), Some(ViewColor::Index(9)))));
        assert!(viewer.cell("other", "a", "1", &row).is_none());
        let status = viewer.status(&[("row", "3".to_owned())]);
        assert_eq!(status, Some(Ok("row 3".into())));
    }

    #[test]
    fn wrong_color_is_an_error() {
        assert!(Viewer::parse("return { colors = { header = 'pink' } }").is_err());
    }
}
