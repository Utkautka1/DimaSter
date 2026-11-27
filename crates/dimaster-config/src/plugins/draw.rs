//! Свободное рисование: `draw.lua` возвращает список фигур поверх окна.
//!
//! Круги и графики собираются из этих примитивов на самом Lua (prelude), так
//! что Rust знает только прямоугольник, линию, многоугольник и текст.

use mlua::{Function, Table, Value};

use super::{Plugins, Snapshot};
use crate::parse_color;

// пиксели в обе стороны: отрицательные отсчитываются от правого или нижнего края
const LIMIT: f32 = 20_000.0;

/// Цвет и непрозрачность фигуры.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Paint {
    /// `None` — цвет текста темы.
    pub color: Option<u32>,
    pub opacity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Rect {
        at: (f32, f32),
        size: (f32, f32),
        radius: u32,
        paint: Paint,
    },
    Line {
        from: (f32, f32),
        to: (f32, f32),
        width: f32,
        paint: Paint,
    },
    Polygon {
        points: Vec<(f32, f32)>,
        paint: Paint,
    },
    Text {
        at: (f32, f32),
        text: String,
        paint: Paint,
    },
}

fn number(table: &Table, key: &str, default: f32) -> f32 {
    let value = table.get::<Option<f32>>(key).ok().flatten();
    let value = value.filter(|v| v.is_finite()).unwrap_or(default);
    value.clamp(-LIMIT, LIMIT)
}

fn pair(table: &Table, x: &str, y: &str) -> (f32, f32) {
    (number(table, x, 0.0), number(table, y, 0.0))
}

fn paint(table: &Table) -> Paint {
    let color: Option<String> = table.get("color").ok().flatten();
    Paint {
        color: color.as_deref().and_then(parse_color),
        opacity: number(table, "opacity", 1.0).clamp(0.0, 1.0),
    }
}

// плоский список `{ x1, y1, x2, y2, ... }`
fn points(table: &Table) -> Vec<(f32, f32)> {
    let list: Vec<f32> = table.get("points").unwrap_or_default();
    let pairs = list
        .as_chunks::<2>()
        .0
        .iter()
        .take(dimaster_limits::PLUGIN_POINTS.size());
    pairs
        .map(|p| (p[0].clamp(-LIMIT, LIMIT), p[1].clamp(-LIMIT, LIMIT)))
        .collect()
}

fn line(table: &Table, paint: Paint) -> Shape {
    Shape::Line {
        from: pair(table, "x1", "y1"),
        to: pair(table, "x2", "y2"),
        width: number(table, "width", 1.0).clamp(1.0, 200.0),
        paint,
    }
}

fn shape(table: &Table) -> Option<Shape> {
    let kind: String = table.get("kind").ok()?;
    let paint = paint(table);
    Some(match kind.as_str() {
        "rect" => Shape::Rect {
            at: pair(table, "x", "y"),
            size: pair(table, "width", "height"),
            radius: number(table, "radius", 0.0).max(0.0) as u32,
            paint,
        },
        "line" => line(table, paint),
        "polygon" => Shape::Polygon {
            points: points(table),
            paint,
        },
        "text" => Shape::Text {
            at: pair(table, "x", "y"),
            text: table.get("text").ok()?,
            paint,
        },
        _ => return None,
    })
}

// вложенные списки (круг, график) разворачиваются по порядку
fn collect(table: &Table, depth: u8, out: &mut Vec<Shape>) {
    if table.contains_key("kind").unwrap_or(false) {
        out.extend(shape(table));
        return;
    }
    for value in table.sequence_values::<Value>().flatten() {
        match value {
            Value::Table(inner)
                if depth < 4 && out.len() < dimaster_limits::PLUGIN_SHAPES.size() =>
            {
                collect(&inner, depth + 1, out)
            }
            _ => {}
        }
    }
}

impl Plugins {
    /// Фигуры из `draw.lua`; пусто, если рисовать нечего или функция упала.
    pub fn draw(&self, facts: &[(String, String)], snapshot: Snapshot) -> Vec<Shape> {
        self.prepare_call(snapshot);
        let result = self.info(facts).and_then(|info| {
            let render: Function = self.registry.get("draw")?;
            render.call::<Option<Table>>(info)
        });
        let mut shapes = Vec::new();
        match result {
            Ok(Some(table)) => collect(&table, 0, &mut shapes),
            Ok(None) => {}
            Err(e) => log::warn!("draw.lua: {e}"),
        }
        shapes.truncate(dimaster_limits::PLUGIN_SHAPES.size());
        shapes
    }
}
