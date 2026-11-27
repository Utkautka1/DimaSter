//! Загрузка конфига, слежение за файлом и перевод в типы крейтов.

use dimaster_i18n::t;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use dimaster_config::{Config, CursorStyle, Decorations, config_dir, load_with, translations};
use dimaster_font::{FontError, FontSystem};
use dimaster_grid::{CursorShape, DefaultColors, Picture};
use dimaster_i18n::tr;
use dimaster_image::{Animation, decode};
use dimaster_render::{Appearance, CursorLook};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

pub(crate) fn config_path() -> Option<PathBuf> {
    Some(config_dir()?.join("init.lua"))
}

/// Включает язык из конфига вместе с переводами пользователя.
pub(crate) fn set_language(config: &Config) {
    let code = dimaster_i18n::resolve(&config.language, |name| std::env::var(name).ok());
    let user = config_dir().map_or_else(|| Ok(Vec::new()), |d| translations(&d, &code));
    let user = user.unwrap_or_else(|e| {
        log::error!("lang/{code}.lua: {e}");
        Vec::new()
    });
    dimaster_i18n::set_language(&code, user);
}

/// Конфиг пользователя; `Err` — текст ошибки, который стоит показать.
pub(crate) fn read() -> Result<Config, String> {
    read_with(&[])
}

/// То же с таблицами, установленными плагинами через `dimaster.set`.
pub(crate) fn read_with(overrides: &[String]) -> Result<Config, String> {
    // скин — первый слой: то, что ставят плагины, важнее его
    let chosen = crate::skins::chosen_layers().into_iter();
    let layers: Vec<String> = chosen.chain(overrides.iter().cloned()).collect();
    read_plain(&layers)
}

/// Настройки без выбранного скина.
pub(crate) fn read_plain(overrides: &[String]) -> Result<Config, String> {
    let Some(path) = config_path() else {
        return Ok(Config::default());
    };
    load_with(&path, overrides).map_err(|e| e.to_string())
}

/// При ошибке в конфиге запускаемся с настройками по умолчанию.
pub(crate) fn read_or_default() -> Config {
    read().unwrap_or_else(|e| {
        log::error!(
            "{}",
            t!(
                "Ошибка конфига, используются значения по умолчанию: {error}",
                error = e
            )
        );
        Config::default()
    })
}

/// Следит за каталогом конфига (редакторы заменяют файл целиком).
pub(crate) fn watch(changed: impl Fn() + Send + 'static) -> Option<RecommendedWatcher> {
    let directory = config_dir()?;
    start_watcher(&directory, changed)
        .inspect_err(|e| {
            log::warn!(
                "{}",
                t!(
                    "Перезагрузка конфига на лету недоступна: {error}",
                    error = e
                )
            )
        })
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
    let mut fonts =
        FontSystem::with_settings(config.font_family.as_deref(), size, &config.font_dirs)?;
    fonts.set_style(config.line_height, config.ligatures);
    fonts.set_features(&config.font_features);
    fonts.set_style_families(config.font_styles.clone());
    Ok(fonts)
}

/// Высота своей полосы заголовка в пикселях.
pub(crate) fn title_height(config: &Config, cell_height: u32) -> u32 {
    match config.window.decorations {
        Decorations::Own => cell_height + config.ui.title.padding,
        Decorations::System | Decorations::None => 0,
    }
}

fn cursor_look(config: &Config) -> CursorLook {
    let look = config.cursor_look;
    CursorLook {
        thickness: look.thickness,
        width: look.width,
        height: look.height,
        radius: look.radius,
        opacity: look.opacity,
        color: look.color,
        text_color: look.text_color,
        hollow_unfocused: look.hollow_unfocused,
        trail_color: look.trail_color,
        trail_opacity: look.trail_opacity,
    }
}

pub(crate) fn appearance(config: &Config, cell_height: u32) -> Appearance {
    let theme = config.theme;
    Appearance {
        foreground: theme.foreground,
        background: theme.background,
        cursor: theme.cursor,
        cursor_look: cursor_look(config),
        selection: theme.selection,
        selection_text: theme.selection_text,
        bold: None,
        ansi: theme.ansi,
        padding: config.padding,
        rtl: config.rtl,
        minimum_contrast: config.minimum_contrast,
        opacity: config.window.opacity,
        top: title_height(config, cell_height),
        wallpaper: None,
        palette: Default::default(),
    }
}

// из каталога берётся случайная картинка, иначе — заданный файл
fn background_path(config: &Config) -> Option<PathBuf> {
    let Some(directory) = &config.window.background_dir else {
        return config.window.background_image.clone();
    };
    let pictures: Vec<PathBuf> = std::fs::read_dir(directory)
        .inspect_err(|e| {
            log::warn!(
                "{}",
                t!(
                    "Каталог фонов {path}: {error}",
                    path = directory.display(),
                    error = e
                )
            )
        })
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_picture(p))
        .collect();
    // для выбора обоев хватает младших разрядов времени
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    pictures.get(seed as usize % pictures.len().max(1)).cloned()
}

fn is_picture(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    ["png", "jpg", "jpeg", "gif", "webp"].contains(&extension.to_ascii_lowercase().as_str())
}

/// Декодирует фоновую картинку; ошибка не мешает работе терминала.
pub(crate) fn wallpaper(config: &Config) -> Option<(Arc<Picture>, f32)> {
    let path = &background_path(config)?;
    let data = std::fs::read(path)
        .inspect_err(|e| {
            log::warn!(
                "{}",
                t!("Фон {path}: {error}", path = path.display(), error = e)
            )
        })
        .ok()?;
    Some((picture(decode(&data)?), config.window.background_opacity))
}

/// Картинка для фона с номером, не занятым картинками программ.
pub(crate) fn picture(animation: Animation) -> Arc<Picture> {
    // номера идут с конца диапазона
    static NEXT: AtomicU64 = AtomicU64::new(u64::MAX);
    let serial = NEXT.fetch_sub(1, Ordering::Relaxed);
    Arc::new(Picture::new(serial, animation))
}

pub(crate) fn default_colors(config: &Config) -> DefaultColors {
    DefaultColors {
        foreground: config.theme.foreground,
        background: config.theme.background,
        cursor: config.theme.cursor,
        ansi: config.theme.ansi,
    }
}

pub(crate) fn cursor_shape(style: CursorStyle) -> CursorShape {
    match style {
        CursorStyle::Block => CursorShape::Block,
        CursorStyle::Underline => CursorShape::Underline,
        CursorStyle::Bar => CursorShape::Bar,
    }
}

// переменные для скриптов, запускающих fetch при открытии shell
fn apply_fetch_env(config: &Config, env: &mut Vec<(String, String)>) {
    env.push(("DIMASTER_STARTUP_FETCH".to_owned(), "1".to_owned()));
    if config.smart.fetch_early {
        env.push(("DIMASTER_FETCH_EARLY".to_owned(), "1".to_owned()));
    }
    if let Ok(exe) = std::env::current_exe() {
        let exe = exe.to_string_lossy().into_owned();
        env.push(("DIMASTER_BIN".to_owned(), exe));
    }
}

fn apply_smart_env(config: &Config, env: &mut Vec<(String, String)>) {
    if config.smart.not_found {
        env.push(("DIMASTER_NOT_FOUND".to_owned(), tr("команда не найдена")));
    }
    if config.smart.startup_fetch {
        apply_fetch_env(config, env);
    }
    if config.smart.color_nano {
        env.push(("DIMASTER_COLOR_NANO".to_owned(), "1".to_owned()));
    }
    if config.smart.color_ls {
        env.push(("DIMASTER_COLOR_LS".to_owned(), "1".to_owned()));
        let has_ls =
            env.iter().any(|(k, _)| k == "LS_COLORS") || std::env::var_os("LS_COLORS").is_some();
        if !has_ls {
            env.push((
                "LS_COLORS".to_owned(),
                crate::syntax::DEFAULT_LS_COLORS.to_owned(),
            ));
        }
    }
}

/// Переменные для shell: из конфига и то, что нужно скриптам интеграции.
pub(crate) fn shell_env(config: &Config) -> Vec<(String, String)> {
    let mut env = config.env.clone();
    if !env.iter().any(|(k, _)| k == "STARSHIP_CONFIG")
        && let Some(starship) = config_dir().map(|d| d.join("starship.toml"))
        && starship.is_file()
    {
        env.push((
            "STARSHIP_CONFIG".to_owned(),
            starship.to_string_lossy().into_owned(),
        ));
    }
    apply_smart_env(config, &mut env);
    // starship читает копию, которую терминал красит под скин
    if let Some(copy) = crate::prompt::redirected(config) {
        env.retain(|v| v.0 != "STARSHIP_CONFIG");
        env.push((
            "STARSHIP_CONFIG".to_owned(),
            copy.to_string_lossy().into_owned(),
        ));
    }
    env
}

/// Команда shell: из конфига, иначе `$SHELL`.
pub(crate) fn shell(config: &Config) -> Vec<String> {
    if !config.shell.is_empty() {
        return config.shell.clone();
    }
    vec![std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_fetch_env_follows_config() {
        let mut config = Config::default();
        let enabled = shell_env(&config);
        assert!(
            enabled
                .iter()
                .any(|(k, v)| k == "DIMASTER_STARTUP_FETCH" && v == "1")
        );
        assert!(
            enabled
                .iter()
                .any(|(k, v)| k == "DIMASTER_COLOR_LS" && v == "1")
        );
        config.smart.startup_fetch = false;
        config.smart.color_ls = false;
        let disabled = shell_env(&config);
        assert!(!disabled.iter().any(|(k, _)| k == "DIMASTER_STARTUP_FETCH"));
        assert!(!disabled.iter().any(|(k, _)| k == "DIMASTER_COLOR_LS"));
    }
}
