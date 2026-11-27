//! Формат Ghostty: `ключ = значение`; темы — файлы того же формата.

use dimaster_i18n::t;
use std::path::Path;

use crate::{Config, CursorStyle, Decorations, parse_color};

const THEME_DIR: &str = "/usr/share/ghostty/themes";

fn pairs(text: &str) -> impl Iterator<Item = (&str, &str)> {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim(), v.trim().trim_matches('"')))
}

pub(crate) fn apply(text: &str, directory: &Path, config: &mut Config, depth: u8) {
    // тема применяется первой: явные цвета конфига должны её перекрывать
    for (key, value) in pairs(text).filter(|(k, _)| *k == "theme") {
        include_theme(key, value, directory, config, depth);
    }
    for (key, value) in pairs(text) {
        match key {
            "config-file" if depth > 0 => {
                let path = super::resolve(directory, value.trim_start_matches('?'));
                if let Ok(included) = std::fs::read_to_string(path) {
                    apply(&included, directory, config, depth - 1);
                }
            }
            _ => setting(key, value, config),
        }
    }
}

fn include_theme(key: &str, value: &str, directory: &Path, config: &mut Config, depth: u8) {
    // «light:X,dark:Y» — берём тёмный вариант
    let name = value
        .split(',')
        .find_map(|p| p.trim().strip_prefix("dark:"))
        .unwrap_or(value);
    let found = [
        directory.join("themes").join(name),
        Path::new(THEME_DIR).join(name),
    ]
    .iter()
    .find_map(|p| std::fs::read_to_string(p).ok());
    match found {
        Some(theme) if depth > 0 => apply(&theme, directory, config, depth - 1),
        _ => log::warn!(
            "{}",
            t!(
                "{key} = {value}: тема Ghostty не найдена",
                key = key,
                value = value
            )
        ),
    }
}

fn color(value: &str, target: &mut u32) {
    if let Some(color) = parse_color(value) {
        *target = color;
    }
}

fn setting(key: &str, value: &str, config: &mut Config) {
    let theme = &mut config.theme;
    match key {
        "foreground" => color(value, &mut theme.foreground),
        "background" => color(value, &mut theme.background),
        "cursor-color" => color(value, &mut theme.cursor),
        "selection-background" => color(value, &mut theme.selection),
        "selection-foreground" => theme.selection_text = parse_color(value),
        "palette" => palette(value, &mut theme.ansi),
        _ => layout(key, value, config),
    }
}

// `palette = 4=#89b4fa`; индексы выше 15 у нас не настраиваются
fn palette(value: &str, ansi: &mut [u32; 16]) {
    let Some((index, hex)) = value.split_once('=') else {
        return;
    };
    let slot = index
        .trim()
        .parse::<usize>()
        .ok()
        .and_then(|i| ansi.get_mut(i));
    if let Some(slot) = slot {
        color(hex, slot);
    }
}

fn layout(key: &str, value: &str, config: &mut Config) {
    match key {
        // повторные font-family — запасные шрифты, главный первый
        "font-family" if config.font_family.is_none() => {
            config.font_family = Some(value.to_owned());
        }
        "font-family-bold" => config.font_styles[0] = Some(value.to_owned()),
        "font-family-italic" => config.font_styles[1] = Some(value.to_owned()),
        "font-family-bold-italic" => config.font_styles[2] = Some(value.to_owned()),
        "font-size" => config.font_size = value.parse().unwrap_or(config.font_size),
        "font-feature" => config.font_features.push(value.to_owned()),
        "minimum-contrast" => {
            if let Ok(v) = value.parse::<f32>() {
                config.minimum_contrast = v.clamp(1.0, 21.0);
            }
        }
        "cursor-style" => config.cursor = CursorStyle::parse(value).unwrap_or(config.cursor),
        "cursor-style-blink" => config.effects.cursor_blink = value == "true",
        _ => window_and_behavior(key, value, config),
    }
}

fn window_and_behavior(key: &str, value: &str, config: &mut Config) {
    match key {
        "window-padding-x" => config.padding.0 = value.parse().unwrap_or(config.padding.0),
        "window-padding-y" => config.padding.1 = value.parse().unwrap_or(config.padding.1),
        "window-width" => config.window.columns = value.parse().ok(),
        "window-height" => config.window.rows = value.parse().ok(),
        "background-opacity" => {
            config.window.opacity = value
                .parse::<f32>()
                .unwrap_or(1.0)
                .max(dimaster_limits::opacity_range().0)
                .min(dimaster_limits::opacity_range().1);
        }
        "background-blur-radius" | "background-blur" => {
            config.window.blur = !matches!(value, "0" | "false");
        }
        "window-decoration" if matches!(value, "false" | "none") => {
            config.window.decorations = Decorations::None;
        }
        _ => behavior_flags(key, value, config),
    }
}

fn behavior_flags(key: &str, value: &str, config: &mut Config) {
    match key {
        "scrollback-limit" => {
            if let Ok(v) = value.parse::<usize>() {
                config.scrollback = v.min(dimaster_limits::SCROLLBACK_MAX.size());
            }
        }
        "clipboard-write" => config.clipboard_write = !matches!(value, "deny" | "false"),
        "clipboard-paste-protection" => config.clipboard_paste_protection = value != "false",
        "mouse-hide-while-typing" => config.mouse_hide_while_typing = value == "true",
        "confirm-close-surface" => config.confirm_close = value != "false",
        "command" => config.shell = value.split_whitespace().map(str::to_owned).collect(),
        _ => {}
    }
}
