//! Генерация файлов со значениями.

use dimaster_i18n::tr;
use dimaster_limits::{ALL, Limit};
use std::fmt::Write;
use std::path::Path;

use crate::color::format_color;
use crate::{Chord, Config, Help, Native, Smart, Theme, Tuning};

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
         padding = {{ x = {x}, y = {y} }},\n  minimum_contrast = {},\n  cursor = {},\n}}\n",
        config.minimum_contrast,
        cursor(config),
    )
}

// только форма — строкой; изменённый вид курсора — таблицей
fn cursor(config: &Config) -> String {
    let (shape, look) = (quote(config.cursor.name()), config.cursor_look);
    if look == crate::CursorLook::default() {
        return shape;
    }
    format!(
        "{{ shape = {shape}, pulse = {}, pulse_period = {}, pulse_min = {}, thickness = {}, height = {} }}",
        look.pulse, look.pulse_period, look.pulse_min, look.thickness, look.height
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
         features = {},\n    dirs = {},\n  }},\n}}\n",
        style_families(config),
        config.font_size,
        config.line_height,
        config.ligatures,
        list(&config.font_features, |f| quote(f)),
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
fn smart(s: &Smart, scrollbar: bool) -> String {
    format!(
        "  scrollbar = {},\n  smart = {{\n    integration = {},\n    startup_fetch = {},\n    fetch_early = {},\n    syntax_highlight = {},\n    color_ls = {},\n    color_nano = {},\n    autosuggest = {},\n    autopair = {},\n    not_found = {},\n    ok = {},\n    empty = {},\n    \
         errors = {},\n    search_tools = {},\n    long_command = {},\n    glow = {},\n    \
         glow_color = {},\n    notify = {},\n    double_alt_sudo = {},\n  }},\n",
        scrollbar,
        s.integration,
        s.startup_fetch,
        s.fetch_early,
        s.syntax_highlight,
        s.color_ls,
        s.color_nano,
        s.autosuggest,
        s.autopair,
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

fn help_texts(h: &Help) -> String {
    format!(
        "    typo = {},\n    sudo = {},\n    chmod = {},\n    missing = {},\n    similar = {},\n",
        quote(&h.typo),
        quote(&tr(&h.sudo)),
        quote(&tr(&h.chmod)),
        quote(&tr(&h.missing)),
        quote(&tr(&h.similar)),
    )
}

// стандартный список двуязычный, поэтому в файл не пишется
fn denied(h: &Help) -> String {
    match h.denied == Help::default().denied {
        true => "    -- denied = { \"permission denied\" },\n".to_owned(),
        false => format!("    denied = {},\n", list(&h.denied, |t| quote(t))),
    }
}

fn help_lists(h: &Help) -> String {
    let pair = |m: &(String, String)| format!("{{ {}, {} }}", quote(&m.0), quote(&m.1));
    format!(
        "    welcome = {},\n{}    skip = {},\n    managers = {},\n",
        list(&h.welcome, |t| quote(&tr(t))),
        denied(h),
        list(&h.skip, |t| quote(t)),
        list(&h.managers, pair),
    )
}

// приветствие и советы при ошибке
fn help(h: &Help) -> String {
    format!(
        "  help = {{\n    enabled = {},\n    duration = {},\n{}{}  }},\n",
        h.enabled,
        h.duration,
        help_texts(h),
        help_lists(h),
    )
}

fn tuning_numbers(t: &Tuning) -> String {
    format!(
        "    multi_click = {},\n    drag_scroll_step = {},\n    double_alt = {},\n    glow_period = {},\n    bell_flash = {},\n    autosave = {},\n    history_commands = {},\n    history_dirs = {},\n    picker_files = {},\n    picker_depth = {},\n",
        t.multi_click,
        t.drag_scroll_step,
        t.double_alt,
        t.glow_period,
        t.bell_flash,
        t.autosave,
        t.history_commands,
        t.history_dirs,
        t.picker_files,
        t.picker_depth,
    )
}

// клавиша в том виде, в каком её принимает `tuning`
fn chord_text(chord: &Chord) -> String {
    let m = chord.mods;
    let held = [
        (m.ctrl, "ctrl+"),
        (m.shift, "shift+"),
        (m.alt, "alt+"),
        (m.logo, "super+"),
    ];
    let mods: String = held.iter().filter(|h| h.0).map(|h| h.1).collect();
    quote(&format!("{mods}{}", chord.key))
}

fn tuning_extra(t: &Tuning) -> String {
    format!(
        "    suggest_line = {},\n    suggest_word = {},\n    memory_commands = {},\n    memory_lines = {},\n    minimap_margin = {},\n    resize_edge = {},\n    video_width = {},\n    video_fps = {},\n    docker_timeout = {},\n",
        list(&t.suggest_line, chord_text),
        list(&t.suggest_word, chord_text),
        t.memory_commands,
        t.memory_lines,
        t.minimap_margin,
        t.resize_edge,
        t.video_width,
        t.video_fps,
        t.docker_timeout,
    )
}

// тайминги и пределы
fn tuning(t: &Tuning) -> String {
    let pair = |p: &(char, char)| quote(&String::from_iter([p.0, p.1]));
    format!(
        "  tuning = {{\n{}    picker_skip = {},\n    hint_letters = {},\n    pairs = {},\n    \
         presentation_zoom = {},\n    restored_marker = {},\n{}  }},\n",
        tuning_numbers(t),
        list(&t.picker_skip, |s| quote(s)),
        quote(&t.hint_letters),
        list(&t.pairs, pair),
        t.presentation_zoom,
        quote(&tr(&t.restored_marker)),
        tuning_extra(t),
    )
}

// изменённые строки действуют, остальные показывают, что вообще можно менять
fn limit_line(limit: &Limit, chosen: &[(String, u64)]) -> String {
    let found = chosen.iter().find(|c| c.0 == limit.name);
    let (mark, value) = match found {
        Some(c) => ("", c.1),
        None => ("-- ", limit.default),
    };
    format!(
        "    -- {}\n    {mark}{} = {value},\n",
        tr(limit.risk),
        limit.name
    )
}

fn limits(chosen: &[(String, u64)]) -> String {
    let lines: String = ALL.iter().map(|l| limit_line(l, chosen)).collect();
    let title = tr("  -- защитные пределы: раскомментируйте строку, чтобы изменить");
    format!("{title}\n  limits = {{\n{lines}  }},\n")
}

fn dictionary(pairs: &[(String, String)]) -> String {
    list(pairs, |p| format!("[{}] = {}", quote(&p.0), quote(&p.1)))
}

fn native_keys(n: &Native) -> String {
    format!(
        "    history_back = {},\n    history_forward = {},\n    complete_keys = {},\n",
        list(&n.history_back, chord_text),
        list(&n.history_forward, chord_text),
        list(&n.complete_keys, chord_text),
    )
}

// удобства shell силами терминала
fn native(n: &Native) -> String {
    format!(
        "  native = {{\n    history_search = {},\n    shell_history = {},\n    complete = {},\n    man_flags = {},\n{}    \
         path_commands = {},\n    abbreviations = {},\n    aliases = {},\n  }},\n",
        n.history_search,
        n.shell_history,
        n.complete,
        n.man_flags,
        native_keys(n),
        list(&n.path_commands, |c| quote(c)),
        dictionary(&n.abbreviations),
        dictionary(&n.aliases),
    )
}

// мышь и буфер обмена
fn interaction(config: &Config) -> String {
    format!(
        "  -- выделенное мышью: primary, clipboard или off\n  copy_on_select = {},\n  \
         scroll_multiplier = {},\n  -- программы могут писать в буфер обмена (OSC 52)\n  \
         clipboard_write = {},\n  clipboard_paste_protection = {},\n  \
         mouse_hide_while_typing = {},\n  confirm_close = {},\n",
        quote(config.copy_on_select.name()),
        config.scroll_multiplier,
        config.clipboard_write,
        config.clipboard_paste_protection,
        config.mouse_hide_while_typing,
        config.confirm_close,
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
        interaction(config)
            + &smart(&config.smart, config.scrollbar)
            + &help(&config.help)
            + &tuning(&config.tuning)
            + &native(&config.native)
            + &limits(&config.limits)
            + &widgets(config)
            + &environment(config),
        config.remote_control,
        config.restore_session,
        config.session_history,
    )
}
