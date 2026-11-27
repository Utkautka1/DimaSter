//! Внешний вид и движение курсора: ключ `cursor` в виде таблицы.

use mlua::{Table, Value};

use crate::{Config, ConfigError, CursorStyle, parse_color};

/// Всё о курсоре, кроме самой формы.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorLook {
    // толщина черты и подчёркивания в пикселях, 0 — по шрифту
    pub thickness: u32,
    // доли ширины и высоты ячейки
    pub width: f32,
    pub height: f32,
    pub radius: u32,
    pub opacity: f32,
    /// `None` — цвет курсора из темы.
    pub color: Option<u32>,
    /// Цвет символа под блоком; `None` — цвет фона.
    pub text_color: Option<u32>,
    /// Рамкой, пока окно не в фокусе.
    pub hollow_unfocused: bool,
    // миллисекунды в каждой фазе мигания
    pub blink_interval: u32,
    // «smooth»: доля пути в секунду; «linear»: ячеек в секунду
    pub speed: f32,
    pub linear: bool,
    // число копий в хвосте и с какого прыжка (в ячейках) он появляется
    pub trail_length: u16,
    pub trail_distance: f32,
    pub trail_opacity: f32,
    pub trail_color: Option<u32>,
    /// Курсор растёт из середины вверх и вниз и сжимается обратно.
    pub pulse: bool,
    // миллисекунды полного цикла и наименьшая доля высоты
    pub pulse_period: u32,
    pub pulse_min: f32,
}

impl Default for CursorLook {
    fn default() -> Self {
        Self {
            thickness: 0,
            width: 1.0,
            height: 1.0,
            radius: 0,
            opacity: 1.0,
            color: None,
            text_color: None,
            hollow_unfocused: false,
            blink_interval: 530,
            speed: 28.0,
            linear: false,
            trail_length: 5,
            trail_distance: 1.5,
            trail_opacity: 0.35,
            trail_color: None,
            pulse: false,
            pulse_period: 1000,
            pulse_min: 0.2,
        }
    }
}

type Done = Result<(), ConfigError>;

fn number<T>(table: &Table, key: &str, range: (T, T), target: &mut T) -> Done
where
    T: mlua::FromLua + PartialOrd + Copy,
{
    if let Some(value) = table.get::<Option<T>>(key)? {
        // NaN не проходит ни одно сравнение и остаётся нижней границей
        *target = match value >= range.0 {
            true if value > range.1 => range.1,
            true => value,
            false => range.0,
        };
    }
    Ok(())
}

fn color(table: &Table, key: &str, target: &mut Option<u32>) -> Done {
    if let Some(text) = table.get::<Option<String>>(key)? {
        *target = Some(parse_color(&text).ok_or_else(|| ConfigError::new(key, &text))?);
    }
    Ok(())
}

fn geometry(table: &Table, look: &mut CursorLook) -> Done {
    number(table, "thickness", (0, 100), &mut look.thickness)?;
    number(table, "width", (0.1, 4.0), &mut look.width)?;
    number(table, "height", (0.1, 2.0), &mut look.height)?;
    number(table, "radius", (0, 100), &mut look.radius)?;
    number(table, "opacity", (0.05, 1.0), &mut look.opacity)?;
    color(table, "color", &mut look.color)?;
    color(table, "text_color", &mut look.text_color)?;
    if let Some(hollow) = table.get::<Option<bool>>("hollow_unfocused")? {
        look.hollow_unfocused = hollow;
    }
    Ok(())
}

fn motion(table: &Table, look: &mut CursorLook) -> Done {
    number(
        table,
        "blink_interval",
        (50, 5000),
        &mut look.blink_interval,
    )?;
    number(table, "speed", (1.0, 2000.0), &mut look.speed)?;
    if let Some(name) = table.get::<Option<String>>("easing")? {
        look.linear = match name.as_str() {
            "smooth" => false,
            "linear" => true,
            _ => return Err(ConfigError::new("cursor.easing", &name)),
        };
    }
    number(table, "trail_length", (1, 60), &mut look.trail_length)?;
    number(
        table,
        "trail_distance",
        (0.0, 1000.0),
        &mut look.trail_distance,
    )?;
    number(table, "trail_opacity", (0.02, 1.0), &mut look.trail_opacity)?;
    color(table, "trail_color", &mut look.trail_color)
}

fn pulse(table: &Table, look: &mut CursorLook) -> Done {
    if let Some(on) = table.get::<Option<bool>>("pulse")? {
        look.pulse = on;
    }
    number(table, "pulse_period", (100, 10_000), &mut look.pulse_period)?;
    number(table, "pulse_min", (0.0, 1.0), &mut look.pulse_min)
}

fn shape(name: &str, target: &mut CursorStyle) -> Done {
    *target = CursorStyle::parse(name).ok_or_else(|| ConfigError::new("cursor", name))?;
    Ok(())
}

/// `cursor = "bar"` или `cursor = { shape = "bar", ... }`.
pub(crate) fn apply(value: &Value, config: &mut Config) -> Done {
    match value {
        Value::Nil => Ok(()),
        Value::String(name) => shape(&name.to_str()?, &mut config.cursor),
        Value::Table(table) => {
            if let Some(name) = table.get::<Option<String>>("shape")? {
                shape(&name, &mut config.cursor)?;
            }
            geometry(table, &mut config.cursor_look)?;
            motion(table, &mut config.cursor_look)?;
            pulse(table, &mut config.cursor_look)
        }
        other => Err(ConfigError::new("cursor", other.type_name())),
    }
}
