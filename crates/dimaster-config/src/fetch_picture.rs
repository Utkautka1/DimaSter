//! Таблица `picture` из `fetch.lua`: размер, положение и рамка картинки.

use mlua::{Table, Value};

use crate::ConfigError;

/// Где картинка относительно текста.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PicturePosition {
    #[default]
    Left,
    Right,
    Top,
}

/// Вид картинки; расстояния в ячейках.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PictureStyle {
    /// `None` — по `logo_width`.
    pub width: Option<usize>,
    /// `None` — половина ширины: на глаз это квадрат.
    pub height: Option<usize>,
    /// `cover` обрезает под рамку, `contain` вписывает, `stretch` тянет.
    pub fit: &'static str,
    pub position: PicturePosition,
    // пустые колонки между картинкой и текстом
    pub gap: usize,
    pub top: usize,
    pub left: usize,
    /// Символы рамки: углы ╭╮╰╯ по порядку, горизонталь и вертикаль.
    pub border: Option<[char; 6]>,
    /// Имя цвета, как в `color()`.
    pub border_color: String,
}

impl Default for PictureStyle {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            fit: "cover",
            position: PicturePosition::Left,
            gap: 3,
            top: 0,
            left: 0,
            border: None,
            border_color: "white".to_owned(),
        }
    }
}

fn cells(table: &Table, key: &str) -> mlua::Result<Option<usize>> {
    let value: Option<usize> = table.get(key)?;
    Ok(value.map(|v| v.min(dimaster_limits::FETCH_PICTURE_CELLS.size())))
}

// название стиля или шесть своих символов
fn border(value: Value) -> Option<Option<[char; 6]>> {
    let Value::String(name) = value else {
        return value.is_nil().then_some(None);
    };
    let name = name.to_str().ok()?.to_owned();
    let chars = match name.as_str() {
        "none" => return Some(None),
        "rounded" => "╭╮╰╯─│",
        "single" => "┌┐└┘─│",
        "double" => "╔╗╚╝═║",
        "heavy" => "┏┓┗┛━┃",
        custom => custom,
    };
    let list: Vec<char> = chars.chars().filter(|c| !c.is_control()).collect();
    list.try_into().ok().map(Some)
}

fn word<T: Copy>(
    table: &Table,
    key: &str,
    words: &[(&str, T)],
    default: T,
) -> Result<T, ConfigError> {
    let Some(text) = table.get::<Option<String>>(key)? else {
        return Ok(default);
    };
    let found = words.iter().find(|w| w.0 == text);
    Ok(found.ok_or_else(|| ConfigError::new(key, &text))?.1)
}

fn choices(table: &Table, style: &mut PictureStyle) -> Result<(), ConfigError> {
    let fits = [
        ("cover", "cover"),
        ("contain", "contain"),
        ("stretch", "stretch"),
    ];
    style.fit = word(table, "fit", &fits, style.fit)?;
    let positions = [
        ("left", PicturePosition::Left),
        ("right", PicturePosition::Right),
        ("top", PicturePosition::Top),
    ];
    style.position = word(table, "position", &positions, style.position)?;
    style.border = border(table.get("border")?)
        .ok_or_else(|| ConfigError::new("picture.border", "6 символов или имя стиля"))?;
    Ok(())
}

pub(crate) fn parse(table: Option<Table>) -> Result<PictureStyle, ConfigError> {
    let mut style = PictureStyle::default();
    let Some(table) = table else { return Ok(style) };
    style.width = cells(&table, "width")?.map(|w| w.max(1));
    style.height = cells(&table, "height")?.map(|h| h.max(1));
    style.gap = cells(&table, "gap")?.unwrap_or(style.gap);
    style.top = cells(&table, "top")?.unwrap_or(0);
    style.left = cells(&table, "left")?.unwrap_or(0);
    if let Some(name) = table.get::<Option<String>>("border_color")? {
        style.border_color = name;
    }
    choices(&table, &mut style)?;
    Ok(style)
}
