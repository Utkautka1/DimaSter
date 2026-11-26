//! Загрузка конфига, слежение за файлом и перевод в типы крейтов.

use std::path::{Path, PathBuf};

use dimaster_config::{Config, CursorStyle, config_dir, load};
use dimaster_font::{FontError, FontSystem};
use dimaster_grid::{CursorShape, DefaultColors};
use dimaster_render::Appearance;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub(crate) fn config_path() -> Option<PathBuf> {
    Some(config_dir()?.join("init.lua"))
}

/// Конфиг пользователя; `Err` — текст ошибки, который стоит показать.
pub(crate) fn read() -> Result<Config, String> {
    let Some(path) = config_path() else {
        return Ok(Config::default());
    };
    load(&path).map_err(|e| e.to_string())
}

/// При ошибке в конфиге запускаемся с настройками по умолчанию.
pub(crate) fn read_or_default() -> Config {
    read().unwrap_or_else(|e| {
        log::error!("Ошибка конфига, используются значения по умолчанию: {e}");
        Config::default()
    })
}

/// Следит за каталогом конфига (редакторы заменяют файл целиком).
pub(crate) fn watch(changed: impl Fn() + Send + 'static) -> Option<RecommendedWatcher> {
    let directory = config_dir()?;
    start_watcher(&directory, changed)
        .inspect_err(|e| log::warn!("Перезагрузка конфига на лету недоступна: {e}"))
        .ok()
}

fn start_watcher(
    directory: &Path,
    changed: impl Fn() + Send + 'static,
) -> notify::Result<RecommendedWatcher> {
    std::fs::create_dir_all(directory)?;
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        // события чтения вызвали бы бесконечную перезагрузку
        let relevant = event.is_ok_and(|e| {
            matches!(
                e.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            )
        });
        if relevant {
            changed();
        }
    })?;
    watcher.watch(directory, RecursiveMode::Recursive)?;
    Ok(watcher)
}

/// Кегль в физических пикселях: пункты → CSS-пиксели → масштаб монитора.
pub(crate) fn font_pixels(points: f32, scale: f64) -> f32 {
    (f64::from(points) * 4.0 / 3.0 * scale) as f32
}

pub(crate) fn fonts(config: &Config, scale: f64) -> Result<FontSystem, FontError> {
    let size = font_pixels(config.font_size, scale);
    FontSystem::with_settings(config.font_family.as_deref(), size, &config.font_dirs)
}

pub(crate) fn appearance(config: &Config) -> Appearance {
    let theme = config.theme;
    Appearance {
        foreground: theme.foreground,
        background: theme.background,
        cursor: theme.cursor,
        selection: theme.selection,
        selection_text: theme.selection_text,
        ansi: theme.ansi,
        padding: config.padding,
        rtl: config.rtl,
    }
}

pub(crate) fn default_colors(config: &Config) -> DefaultColors {
    DefaultColors {
        foreground: config.theme.foreground,
        background: config.theme.background,
    }
}

pub(crate) fn cursor_shape(style: CursorStyle) -> CursorShape {
    match style {
        CursorStyle::Block => CursorShape::Block,
        CursorStyle::Underline => CursorShape::Underline,
        CursorStyle::Bar => CursorShape::Bar,
    }
}

/// Команда shell: из конфига, иначе `$SHELL`.
pub(crate) fn shell(config: &Config) -> Vec<String> {
    if !config.shell.is_empty() {
        return config.shell.clone();
    }
    vec![std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())]
}
