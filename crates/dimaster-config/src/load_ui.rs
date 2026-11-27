//! Разбор таблицы `ui`.
//!
//! Неверный цвет или слово — ошибка конфига; числа вне разумного диапазона
//! прижимаются к нему, чтобы опечатка не увела панель за экран.

use mlua::Table;

use crate::ui::{Align, Anchor, PanelStyle, TitleButton, Ui, WindowButton};
use crate::{Action, ConfigError, parse_color};

type Done = Result<(), ConfigError>;

fn set<T: mlua::FromLua>(table: &Table, key: &str, target: &mut T) -> Done {
    if let Some(value) = table.get::<Option<T>>(key)? {
        *target = value;
    }
    Ok(())
}

fn optional_color(table: &Table, key: &str) -> Result<Option<u32>, ConfigError> {
    let Some(text) = table.get::<Option<String>>(key)? else {
        return Ok(None);
    };
    parse_color(&text)
        .map(Some)
        .ok_or_else(|| ConfigError::new(key, &text))
}

fn theme_color(table: &Table, key: &str, target: &mut Option<u32>) -> Done {
    *target = optional_color(table, key)?.or(*target);
    Ok(())
}

fn color(table: &Table, key: &str, target: &mut u32) -> Done {
    *target = optional_color(table, key)?.unwrap_or(*target);
    Ok(())
}

// слово из закрытого списка
fn word<T: Copy>(table: &Table, key: &str, words: &[(&str, T)], target: &mut T) -> Done {
    let Some(text) = table.get::<Option<String>>(key)? else {
        return Ok(());
    };
    let found = words.iter().find(|w| w.0 == text);
    *target = found.ok_or_else(|| ConfigError::new(key, &text))?.1;
    Ok(())
}

fn side(table: &Table, key: &str, left: &mut bool) -> Done {
    word(table, key, &[("left", true), ("right", false)], left)
}

fn section(table: &Table, key: &str, apply: impl FnOnce(&Table) -> Done) -> Done {
    match table.get::<Option<Table>>(key)? {
        Some(inner) => apply(&inner),
        None => Ok(()),
    }
}

fn look(table: &Table, style: &mut PanelStyle) -> Done {
    theme_color(table, "background", &mut style.background)?;
    theme_color(table, "color", &mut style.color)?;
    theme_color(table, "selected_background", &mut style.selected_background)?;
    theme_color(table, "selected_color", &mut style.selected_color)?;
    set(table, "radius", &mut style.radius)?;
    set(table, "padding", &mut style.padding)?;
    set(table, "opacity", &mut style.opacity)?;
    (style.radius, style.padding) = (style.radius.min(200), style.padding.min(100));
    style.opacity = style.opacity.clamp(0.0, 1.0);
    Ok(())
}

fn title_button(entry: &Table) -> Result<TitleButton, ConfigError> {
    let mut kind = None;
    let words = [
        ("minimize", Some(WindowButton::Minimize)),
        ("maximize", Some(WindowButton::Maximize)),
        ("close", Some(WindowButton::Close)),
    ];
    word(entry, "action", &words, &mut kind)?;
    Ok(TitleButton {
        kind: kind.ok_or_else(|| ConfigError::new("buttons.action", "не задано"))?,
        text: entry.get("text")?,
        color: optional_color(entry, "color")?,
    })
}

fn title(table: &Table, ui: &mut Ui) -> Done {
    let style = &mut ui.title;
    set(table, "padding", &mut style.padding)?;
    style.padding = style.padding.min(100);
    set(table, "format", &mut style.format)?;
    let aligns = [
        ("left", Align::Left),
        ("center", Align::Center),
        ("right", Align::Right),
    ];
    word(table, "align", &aligns, &mut style.align)?;
    theme_color(table, "background", &mut style.background)?;
    theme_color(table, "color", &mut style.color)?;
    side(table, "buttons_side", &mut style.buttons_left)?;
    if let Some(list) = table.get::<Option<Vec<Table>>>("buttons")? {
        style.buttons = list
            .iter()
            .take(8)
            .map(title_button)
            .collect::<Result<_, _>>()?;
    }
    Ok(())
}

fn palette_item(entry: &Table) -> Result<(String, Action), ConfigError> {
    let name: String = entry.get("action")?;
    let action = Action::parse(&name).ok_or_else(|| ConfigError::new("palette.action", &name))?;
    Ok((
        entry.get::<Option<String>>("title")?.unwrap_or(name),
        action,
    ))
}

fn palette(table: &Table, ui: &mut Ui) -> Done {
    let style = &mut ui.palette;
    set(table, "width", &mut style.width)?;
    set(table, "rows", &mut style.rows)?;
    (style.width, style.rows) = (style.width.clamp(10, 500), style.rows.clamp(1, 100));
    let anchors = [
        ("top", Anchor::Top),
        ("center", Anchor::Center),
        ("bottom", Anchor::Bottom),
    ];
    word(table, "position", &anchors, &mut style.position)?;
    set(table, "prompt", &mut style.prompt)?;
    if let Some(list) = table.get::<Option<Vec<Table>>>("items")? {
        style.items = list.iter().map(palette_item).collect::<Result<_, _>>()?;
    }
    Ok(())
}

fn history(table: &Table, ui: &mut Ui) -> Done {
    let style = &mut ui.history;
    set(table, "width", &mut style.width)?;
    set(table, "rows", &mut style.rows)?;
    (style.width, style.rows) = (style.width.clamp(0.2, 1.0), style.rows.clamp(1, 100));
    side(table, "side", &mut style.left)
}

fn marks(table: &Table, ui: &mut Ui) -> Done {
    let marks = &mut ui.marks;
    color(table, "hint_color", &mut marks.hint_color)?;
    color(table, "hint_background", &mut marks.hint_background)?;
    color(table, "found", &mut marks.found)?;
    color(table, "current", &mut marks.current)?;
    color(table, "link", &mut marks.link)
}

fn strip(table: &Table, strip: &mut crate::ui::Strip) -> Done {
    set(table, "width", &mut strip.width)?;
    strip.width = strip.width.clamp(1, 400);
    color(table, "color", &mut strip.color)?;
    color(table, "command", &mut strip.command)?;
    color(table, "error", &mut strip.error)?;
    color(table, "found", &mut strip.found)?;
    side(table, "side", &mut strip.left)
}

fn monitor(table: &Table, ui: &mut Ui) -> Done {
    let style = &mut ui.monitor;
    side(table, "side", &mut style.left)?;
    word(
        table,
        "position",
        &[("top", false), ("bottom", true)],
        &mut style.bottom,
    )?;
    set(table, "format", &mut style.format)?;
    set(table, "waiting", &mut style.waiting)
}

// любой ключ вида отделяет плашку от общих панелей, начиная с их значений
fn notice_look(table: &Table, ui: &mut Ui) -> Done {
    let keys = ["background", "color", "radius", "padding", "opacity"];
    if keys.iter().any(|k| table.contains_key(*k).unwrap_or(false)) {
        let mut own = ui.panel;
        look(table, &mut own)?;
        ui.notice.look = Some(own);
    }
    Ok(())
}

// число — по обеим осям, таблица { x, y } — раздельно
fn margin(table: &Table, target: &mut (u32, u32)) -> Done {
    match table.get::<mlua::Value>("margin")? {
        mlua::Value::Nil => Ok(()),
        mlua::Value::Table(pair) => {
            *target = (pair.get("x")?, pair.get("y")?);
            Ok(())
        }
        _ => {
            let both: u32 = table.get("margin")?;
            *target = (both, both);
            Ok(())
        }
    }
}

fn notice(table: &Table, ui: &mut Ui) -> Done {
    notice_look(table, ui)?;
    let style = &mut ui.notice;
    set(table, "copy", &mut style.copy)?;
    set(table, "text", &mut style.text)?;
    set(table, "duration", &mut style.duration)?;
    style.duration = style.duration.clamp(200, 60_000);
    let sides = [
        ("left", Align::Left),
        ("center", Align::Center),
        ("right", Align::Right),
    ];
    word(table, "side", &sides, &mut style.side)?;
    let anchors = [
        ("top", Anchor::Top),
        ("center", Anchor::Center),
        ("bottom", Anchor::Bottom),
    ];
    word(table, "position", &anchors, &mut style.position)?;
    margin(table, &mut style.margin)?;
    style.margin = (style.margin.0.min(2000), style.margin.1.min(2000));
    Ok(())
}

fn texts(table: &Table, ui: &mut Ui) -> Done {
    let texts = &mut ui.texts;
    let fields = [
        ("vi", &mut texts.vi),
        ("hint", &mut texts.hint),
        ("minimap", &mut texts.minimap),
        ("search", &mut texts.search),
        ("search_plain", &mut texts.search_plain),
        ("search_regex", &mut texts.search_regex),
        ("history", &mut texts.history),
        ("history_output", &mut texts.history_output),
        ("history_failed", &mut texts.history_failed),
    ];
    fields
        .into_iter()
        .try_for_each(|(key, target)| set(table, key, target))
}

pub(crate) fn apply(table: &Table, ui: &mut Ui) -> Done {
    section(table, "panel", |t| look(t, &mut ui.panel))?;
    section(table, "title", |t| title(t, ui))?;
    section(table, "palette", |t| palette(t, ui))?;
    section(table, "history", |t| history(t, ui))?;
    section(table, "marks", |t| marks(t, ui))?;
    section(table, "scrollbar", |t| strip(t, &mut ui.scrollbar))?;
    section(table, "minimap", |t| strip(t, &mut ui.minimap))?;
    section(table, "monitor", |t| monitor(t, ui))?;
    section(table, "notice", |t| notice(t, ui))?;
    word(
        table,
        "prompt_position",
        &[("top", true), ("bottom", false)],
        &mut ui.prompt_top,
    )?;
    section(table, "texts", |t| texts(t, ui))
}
