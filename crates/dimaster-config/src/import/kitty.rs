//! Формат kitty: `ключ значение`, поддерживаются `include`.

use std::path::Path;

use crate::{Config, CursorStyle, parse_color};

pub(crate) fn apply(text: &str, directory: &Path, config: &mut Config, depth: u8) {
    for line in text.lines().map(str::trim).filter(|l| !l.starts_with('#')) {
        let Some((key, value)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let value = value.trim();
        match key {
            "include" if depth > 0 => {
                let path = super::resolve(directory, value);
                let parent = path.parent().unwrap_or(directory).to_owned();
                if let Ok(included) = std::fs::read_to_string(path) {
                    apply(&included, &parent, config, depth - 1);
                }
            }
            _ => setting(key, value, config),
        }
    }
}

fn color(value: &str, target: &mut u32) {
    if let Some(color) = parse_color(value) {
        *target = color;
    }
}

fn setting(key: &str, value: &str, config: &mut Config) {
    let theme = &mut config.theme;
    if let Some(index) = key
        .strip_prefix("color")
        .and_then(|n| n.parse::<usize>().ok())
    {
        if let Some(slot) = theme.ansi.get_mut(index) {
            color(value, slot);
        }
        return;
    }
    match key {
        "foreground" => color(value, &mut theme.foreground),
        "background" => color(value, &mut theme.background),
        "cursor" => color(value, &mut theme.cursor),
        "selection_background" => color(value, &mut theme.selection),
        "selection_foreground" => theme.selection_text = parse_color(value),
        _ => layout(key, value, config),
    }
}

fn layout(key: &str, value: &str, config: &mut Config) {
    match key {
        "font_family" if value != "monospace" => config.font_family = Some(family(value)),
        "font_size" => config.font_size = value.parse().unwrap_or(config.font_size),
        "window_padding_width" => {
            // kitty задаёт отступ в пунктах, у нас пиксели
            if let Ok(points) = value.parse::<f32>() {
                let pixels = (points.max(0.0) * 4.0 / 3.0).round() as u32;
                config.padding = (pixels, pixels);
            }
        }
        "scrollback_lines" => config.scrollback = value.parse().unwrap_or(config.scrollback),
        "cursor_shape" => config.cursor = CursorStyle::parse(value).unwrap_or(config.cursor),
        "shell" if value != "." => {
            config.shell = value.split_whitespace().map(str::to_owned).collect();
        }
        _ => {}
    }
}

// kitty пишет начертание в конце имени, а нам нужно только семейство
fn family(value: &str) -> String {
    let styles = [" Bold", " Italic", " Regular", " Medium", " Light"];
    let mut name = value.trim_matches('"');
    while let Some(shorter) = styles.iter().find_map(|s| name.strip_suffix(s)) {
        name = shorter;
    }
    name.to_owned()
}
