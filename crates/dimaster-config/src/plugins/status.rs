//! Строка статуса: вызов Lua-функции и разбор того, что она вернула.

use mlua::{Function, Table, Value};

use super::{Plugins, Snapshot};
use crate::parse_color;

const MAX_ROWS: usize = 5;
const MAX_SEGMENTS: usize = 32;
// пиксели: защита от опечатки, которая заняла бы всё окно
const MAX_PIXELS: u32 = 200;

/// Кусок строки.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub text: String,
    /// `None` — цвет текста темы.
    pub color: Option<u32>,
    pub background: Option<u32>,
    // любые символы по краям подложки; пустая строка — край ровный
    pub left_cap: String,
    pub right_cap: String,
    // скругление подложки в пикселях
    pub radius: u32,
    pub opacity: f32,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct StatusLine {
    pub left: Vec<Segment>,
    pub center: Vec<Segment>,
    pub right: Vec<Segment>,
}

/// Форма и положение всего бара; расстояния в пикселях.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarStyle {
    pub top: bool,
    /// Отступ бара от краёв окна: по горизонтали и вертикали.
    pub margin: (u32, u32),
    /// Поля внутри подложки.
    pub padding: (u32, u32),
    // зазор между строками бара
    pub gap: u32,
    pub background: Option<u32>,
    pub radius: u32,
    pub opacity: f32,
}

impl Default for BarStyle {
    fn default() -> Self {
        Self {
            top: false,
            margin: (12, 10),
            padding: (0, 0),
            gap: 4,
            background: None,
            radius: 0,
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatusBar {
    pub style: BarStyle,
    pub rows: Vec<StatusLine>,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self {
            style: BarStyle::default(),
            rows: vec![StatusLine::default()],
        }
    }
}

fn plain(text: String) -> Segment {
    Segment {
        text,
        color: None,
        background: None,
        left_cap: String::new(),
        right_cap: String::new(),
        radius: 0,
        opacity: 1.0,
    }
}

fn color(table: &Table, key: &str) -> Option<u32> {
    let text: Option<String> = table.get(key).ok()?;
    text.as_deref().and_then(parse_color)
}

fn pixels(table: &Table, key: &str, default: u32) -> u32 {
    let value = table.get::<Option<u32>>(key).ok().flatten();
    value.unwrap_or(default).min(MAX_PIXELS)
}

fn opacity(table: &Table) -> f32 {
    let value = table.get::<Option<f32>>("opacity").ok().flatten();
    value.unwrap_or(1.0).clamp(0.0, 1.0)
}

// `rounded = true` — сокращение для полукругов Powerline
fn caps(table: &Table) -> (String, String) {
    let rounded = table
        .get::<Option<bool>>("rounded")
        .ok()
        .flatten()
        .unwrap_or(false);
    let cap = |key, round: &str| {
        let own = table.get::<Option<String>>(key).ok().flatten();
        let fallback = if rounded {
            round.to_owned()
        } else {
            String::new()
        };
        own.map_or(fallback, |c| c.chars().take(4).collect())
    };
    (cap("left_cap", "\u{e0b6}"), cap("right_cap", "\u{e0b4}"))
}

fn styled(table: &Table) -> Option<Segment> {
    let (left_cap, right_cap) = caps(table);
    Some(Segment {
        color: color(table, "color"),
        background: color(table, "background"),
        left_cap,
        right_cap,
        radius: pixels(table, "radius", 0),
        opacity: opacity(table),
        ..plain(table.get("text").ok()?)
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
        .take(MAX_SEGMENTS)
        .collect()
}

fn line(table: &Table) -> StatusLine {
    StatusLine {
        left: side(table, "left"),
        center: side(table, "center"),
        right: side(table, "right"),
    }
}

// число — одинаково по обеим осям, таблица { x, y } — раздельно
fn pair(table: &Table, key: &str, default: (u32, u32)) -> (u32, u32) {
    match table.get::<Value>(key) {
        Ok(Value::Table(t)) => (pixels(&t, "x", default.0), pixels(&t, "y", default.1)),
        Ok(Value::Integer(_)) => {
            let both = pixels(table, key, default.0);
            (both, both)
        }
        _ => default,
    }
}

fn style(table: &Table) -> BarStyle {
    let base = BarStyle::default();
    let position: Option<String> = table.get("position").ok().flatten();
    BarStyle {
        top: position.as_deref() == Some("top"),
        margin: pair(table, "margin", base.margin),
        padding: pair(table, "padding", base.padding),
        gap: pixels(table, "gap", base.gap),
        background: color(table, "background"),
        radius: pixels(table, "radius", base.radius),
        opacity: opacity(table),
    }
}

// `rows = { {...}, {...} }` — несколько строк, иначе сама таблица и есть строка
fn rows(table: &Table) -> Vec<StatusLine> {
    let Ok(Some(list)) = table.get::<Option<Table>>("rows") else {
        return vec![line(table)];
    };
    list.sequence_values::<Table>()
        .flatten()
        .take(MAX_ROWS)
        .map(|row| line(&row))
        .collect()
}

impl Plugins {
    /// `None`, если функция не зарегистрирована или завершилась ошибкой.
    pub fn status(&self, facts: &[(String, String)], snapshot: Snapshot) -> Option<StatusBar> {
        self.prepare_call(snapshot);
        let info = self.info(facts).ok()?;
        let render: Function = self.registry.get("status").ok()?;
        let table: Table = render
            .call::<Option<Table>>(info)
            .inspect_err(|e| log::warn!("statusbar: {e}"))
            .ok()??;
        Some(StatusBar {
            style: style(&table),
            rows: rows(&table),
        })
    }
}
