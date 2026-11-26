//! Строка статуса: вызов Lua-функции и разбор того, что она вернула.

use mlua::{Function, Table, Value};

use super::{Plugins, Snapshot};
use crate::parse_color;

/// Кусок строки одного цвета.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    /// `None` — цвет текста темы.
    pub color: Option<u32>,
    pub background: Option<u32>,
    /// Скруглённые края «таблетки» (нужен Nerd Font).
    pub rounded: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StatusLine {
    pub left: Vec<Segment>,
    pub center: Vec<Segment>,
    pub right: Vec<Segment>,
}

fn plain(text: String) -> Segment {
    Segment {
        text,
        color: None,
        background: None,
        rounded: false,
    }
}

fn styled(table: &Table) -> Option<Segment> {
    let color = |key| {
        let text: Option<String> = table.get(key).ok()?;
        text.as_deref().and_then(parse_color)
    };
    Some(Segment {
        text: table.get("text").ok()?,
        color: color("color"),
        background: color("background"),
        rounded: table.get::<Option<bool>>("rounded").ok()?.unwrap_or(false),
    })
}

fn segment(value: Value) -> Option<Segment> {
    match value {
        Value::String(text) => Some(plain(text.to_str().ok()?.to_owned())),
        Value::Table(table) => styled(&table),
        _ => None,
    }
}

fn side(table: &Table, key: &str) -> Vec<Segment> {
    let Ok(Some(list)) = table.get::<Option<Table>>(key) else {
        return Vec::new();
    };
    list.sequence_values::<Value>()
        .flatten()
        .filter_map(segment)
        .take(32)
        .collect()
}

impl Plugins {
    /// `None`, если функция не зарегистрирована или завершилась ошибкой.
    pub fn status(&self, facts: &[(String, String)], snapshot: Snapshot) -> Option<StatusLine> {
        self.prepare_call(snapshot);
        let info = self.lua.create_table().ok()?;
        for (name, value) in facts {
            info.set(name.as_str(), value.as_str()).ok()?;
        }
        let render: Function = self.registry.get("status").ok()?;
        let table: Table = render
            .call::<Option<Table>>(info)
            .inspect_err(|e| log::warn!("statusbar: {e}"))
            .ok()??;
        Some(StatusLine {
            left: side(&table, "left"),
            center: side(&table, "center"),
            right: side(&table, "right"),
        })
    }
}
