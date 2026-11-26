//! Генерация файлов со значениями.

use std::fmt::Write;
use std::path::Path;

use crate::color::format_color;
use crate::{Config, Theme};

// строка Lua: экранируются только символы, ломающие литерал
fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' | '\\' => out.extend(['\\', ch]),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }
    out + "\""
}

fn path(value: &Path) -> String {
    quote(&value.to_string_lossy())
}

fn list<T>(items: &[T], render: impl Fn(&T) -> String) -> String {
    let parts: Vec<_> = items.iter().map(render).collect();
    format!("{{ {} }}", parts.join(", "))
}

pub(super) fn theme(theme: &Theme) -> String {
    let hex = |c: u32| quote(&format_color(c));
    let mut out = format!(
        "-- Цвета, перенесённые из другого терминала.\nreturn {{\n  foreground = {},\n  \
         background = {},\n  cursor = {},\n  selection = {},\n",
        hex(theme.foreground),
        hex(theme.background),
        hex(theme.cursor),
        hex(theme.selection),
    );
    if let Some(color) = theme.selection_text {
        let _ = writeln!(out, "  selection_text = {},", hex(color));
    }
    let ansi = |range: &[u32]| range.iter().map(|&c| hex(c)).collect::<Vec<_>>().join(", ");
    let _ = write!(
        out,
        "  ansi = {{\n    {},\n    {},\n  }},\n}}\n",
        ansi(&theme.ansi[..8]),
        ansi(&theme.ansi[8..])
    );
    out
}

pub(super) fn appearance(config: &Config) -> String {
    let theme = match config.theme == Theme::default() {
        true => "  -- theme = \"nord\", -- или catppuccin-mocha, gruvbox-dark, themes/<имя>.lua\n",
        false => "  theme = \"imported\",\n",
    };
    let (x, y) = config.padding;
    format!(
        "-- Цвета, отступы и курсор.\nreturn {{\n{theme}  -- colors = {{ background = \"#101010\" }},\n  \
         padding = {{ x = {x}, y = {y} }},\n  cursor = {},\n}}\n",
        quote(config.cursor.name()),
    )
}

// отдельные шрифты начертаний или пример в комментарии
fn style_families(config: &Config) -> String {
    let keys = ["bold", "italic", "bold_italic"];
    keys.iter()
        .zip(&config.font_styles)
        .map(|(key, family)| match family {
            Some(name) => format!("    {key} = {},\n", quote(name)),
            None => format!("    -- {key} = \"Victor Mono\",\n"),
        })
        .collect()
}

pub(super) fn font(config: &Config) -> String {
    let family = match &config.font_family {
        Some(name) => format!("family = {},", quote(name)),
        None => "-- family = \"JetBrainsMono Nerd Font\",".to_owned(),
    };
    format!(
        "-- Шрифт. size — в пунктах, dirs — свои каталоги со шрифтами.\nreturn {{\n  font = {{\n    \
         {family}\n{}    size = {},\n    line_height = {},\n    ligatures = {},\n    \
         dirs = {},\n  }},\n}}\n",
        style_families(config),
        config.font_size,
        config.line_height,
        config.ligatures,
        list(&config.font_dirs, |d| path(d)),
    )
}

// необязательное число или закомментированный пример
fn optional(key: &str, value: Option<u32>, example: u32) -> String {
    match value {
        Some(value) => format!("{key} = {value},"),
        None => format!("-- {key} = {example},"),
    }
}

pub(super) fn window(config: &Config) -> String {
    let window = &config.window;
    let image = match &window.background_image {
        Some(file) => format!("background_image = {},", path(file)),
        None => "-- background_image = \"~/Pictures/wall.png\",".to_owned(),
    };
    format!(
        "-- Окно. decorations: own (своя рамка), system, none.\nreturn {{\n  window = {{\n    \
         decorations = {},\n    opacity = {},\n    blur = {},\n    {image}\n    \
         background_opacity = {},\n    {}\n    {}\n    {}\n  }},\n}}\n",
        quote(window.decorations.name()),
        window.opacity,
        window.blur,
        window.background_opacity,
        match &window.background_dir {
            Some(dir) => format!("background_dir = {},", path(dir)),
            None => "-- background_dir = \"~/Pictures/walls\",".to_owned(),
        },
        optional("columns", window.columns, 100),
        optional("rows", window.rows, 30),
    )
}

pub(super) fn effects(config: &Config) -> String {
    let effects = config.effects;
    format!(
        "-- Анимации.\nreturn {{\n  effects = {{\n    cursor_blink = {},\n    \
         cursor_animation = {},\n    cursor_trail = {},\n    smooth_scroll = {},\n  }},\n}}\n",
        effects.cursor_blink, effects.cursor_animation, effects.cursor_trail, effects.smooth_scroll,
    )
}

// умная оболочка: подсказки под командами и реакция на долгие
fn smart(config: &Config) -> String {
    let s = &config.smart;
    format!(
        "  scrollbar = {},\n  smart = {{\n    integration = {},\n    not_found = {},\n    ok = {},\n    empty = {},\n    \
         errors = {},\n    search_tools = {},\n    long_command = {},\n    glow = {},\n    \
         glow_color = {},\n    notify = {},\n    double_alt_sudo = {},\n  }},\n",
        config.scrollbar,
        s.integration,
        s.not_found,
        quote(&s.ok),
        quote(&s.empty),
        s.errors,
        list(&s.search_tools, |t| quote(t)),
        s.long_command,
        s.glow,
        quote(&format_color(s.glow_color)),
        s.notify,
        s.double_alt_sudo,
    )
}

fn widgets(config: &Config) -> String {
    let w = config.widgets;
    format!(
        "  -- строка статуса настраивается в statusbar.lua\n  widgets = {{\n    statusbar = {},\n    \
         interval = {},\n    docker = {},\n    \
         -- git status в чужом репозитории может выполнить его код\n    git_changes = {},\n    monitor = {},\n    minimap = {},\n  }},\n",
        w.statusbar, w.interval, w.docker, w.git_changes, w.monitor, w.minimap,
    )
}

// отсортированная таблица переменных или пример в комментарии
fn environment(config: &Config) -> String {
    if config.env.is_empty() {
        return "  -- env = { STARSHIP_CONFIG = \"~/.config/dimaster/starship.toml\" },\n"
            .to_owned();
    }
    let pairs: Vec<_> = config
        .env
        .iter()
        .map(|(k, v)| format!("[{}] = {}", quote(k), quote(v)))
        .collect();
    format!("  env = {{ {} }},\n", pairs.join(", "))
}

// мышь и буфер обмена
fn interaction(config: &Config) -> String {
    format!(
        "  -- выделенное мышью: primary, clipboard или off\n  copy_on_select = {},\n  \
         scroll_multiplier = {},\n  -- программы могут писать в буфер обмена (OSC 52)\n  \
         clipboard_write = {},\n",
        quote(config.copy_on_select.name()),
        config.scroll_multiplier,
        config.clipboard_write,
    )
}

pub(super) fn behavior(config: &Config) -> String {
    let shell = match config.shell.is_empty() {
        true => "-- shell = { \"/usr/bin/fish\" }, -- по умолчанию $SHELL".to_owned(),
        false => format!("shell = {},", list(&config.shell, |w| quote(w))),
    };
    format!(
        "-- Поведение терминала.\nreturn {{\n  scrollback = {},\n  {shell}\n  images_scroll = {},\n  \
         rtl = {},\n  term = {},\n{}  -- dimaster ctl: управление окнами из командной строки\n  \
         remote_control = {},\n  -- восстановление пишет вывод терминала на диск, поэтому включается явно\n  \
         session = {{ restore = {}, history_lines = {} }},\n}}\n",
        config.scrollback,
        config.images_scroll,
        config.rtl,
        quote(&config.term),
        interaction(config) + &smart(config) + &widgets(config) + &environment(config),
        config.remote_control,
        config.restore_session,
        config.session_history,
    )
}
