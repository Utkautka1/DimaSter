//! Формат Alacritty — TOML.

use dimaster_i18n::t;
use toml::{Table, Value};

use crate::{Config, Theme, parse_color};

const NAMES: [&str; 8] = [
    "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
];

fn find<'a>(table: &'a Table, path: &[&str]) -> Option<&'a Value> {
    let (last, parents) = path.split_last()?;
    parents
        .iter()
        .try_fold(table, |t, key| t.get(*key)?.as_table())?
        .get(*last)
}

fn color(table: &Table, path: &[&str], target: &mut u32) {
    let parsed = find(table, path)
        .and_then(Value::as_str)
        .and_then(|s| parse_color(s.trim_start_matches("0x")));
    if let Some(color) = parsed {
        *target = color;
    }
}

fn selection(table: &Table, theme: &mut Theme) {
    color(
        table,
        &["colors", "selection", "background"],
        &mut theme.selection,
    );
    theme.selection_text = find(table, &["colors", "selection", "text"])
        .and_then(Value::as_str)
        .and_then(|s| parse_color(s.trim_start_matches("0x")));
}

fn colors(table: &Table, config: &mut Config) {
    let theme = &mut config.theme;
    color(
        table,
        &["colors", "primary", "foreground"],
        &mut theme.foreground,
    );
    color(
        table,
        &["colors", "primary", "background"],
        &mut theme.background,
    );
    color(table, &["colors", "cursor", "cursor"], &mut theme.cursor);
    selection(table, theme);
    for (index, name) in NAMES.iter().enumerate() {
        color(table, &["colors", "normal", name], &mut theme.ansi[index]);
        color(
            table,
            &["colors", "bright", name],
            &mut theme.ansi[index + 8],
        );
    }
}

pub(crate) fn apply(text: &str, config: &mut Config) {
    let table: Table = match text.parse() {
        Ok(table) => table,
        Err(e) => return log::warn!("{}", t!("alacritty.toml не разобран: {error}", error = e)),
    };
    colors(&table, config);
    if let Some(family) = find(&table, &["font", "normal", "family"]).and_then(Value::as_str) {
        config.font_family = Some(family.to_owned());
    }
    let size = find(&table, &["font", "size"]);
    if let Some(size) = size.and_then(|v| v.as_float().or(v.as_integer().map(|i| i as f64))) {
        config.font_size = size as f32;
    }
    let lines = find(&table, &["scrolling", "history"]).and_then(Value::as_integer);
    if let Some(lines) = lines.and_then(|v| usize::try_from(v).ok()) {
        config.scrollback = lines;
    }
}
