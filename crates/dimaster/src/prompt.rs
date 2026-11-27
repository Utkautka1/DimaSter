//! Приглашение starship в цветах скина.
//!
//! Starship берёт цвета из своего файла и перечитывает его перед каждым
//! приглашением. Поэтому shell получает копию в каталоге данных, а терминал
//! перекрашивает её; файл пользователя не меняется.

use std::path::PathBuf;

use dimaster_config::{Config, Theme, config_dir};
use dimaster_i18n::t;

use crate::history_store::data_dir;

// меньше такого разброса каналов цвет считается серым
const NEUTRAL: f32 = 60.0;

fn managed() -> Option<PathBuf> {
    Some(data_dir()?.join("starship.toml"))
}

fn usual() -> Vec<PathBuf> {
    let own = config_dir().map(|d| d.join("starship.toml"));
    let inherited = std::env::var_os("STARSHIP_CONFIG").map(PathBuf::from);
    let home = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/starship.toml"));
    [own, inherited, home].into_iter().flatten().collect()
}

// файл пользователя; копия сюда не попадает, иначе она красилась бы из себя
fn source(config: &Config) -> Option<PathBuf> {
    let chosen = config.env.iter().find(|v| v.0 == "STARSHIP_CONFIG");
    let named = chosen.map(|v| PathBuf::from(&v.1));
    let copy = managed();
    let candidates = named.into_iter().chain(usual());
    candidates
        .filter(|path| Some(path) != copy.as_ref())
        .find(|path| path.is_file())
}

/// Куда направить `STARSHIP_CONFIG`. Путь отдаётся всегда: стиль можно
/// выбрать и после запуска shell, а без файла starship берёт свои значения.
pub(crate) fn redirected() -> Option<PathBuf> {
    managed()
}

fn channels(color: u32) -> [f32; 3] {
    [16, 8, 0].map(|shift| ((color >> shift) & 0xff) as f32)
}

// оттенок в градусах и разброс каналов
fn hue(color: u32) -> (f32, f32) {
    let [r, g, b] = channels(color);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let chroma = max - min;
    let sector = match max {
        _ if chroma == 0.0 => 0.0,
        m if m == r => ((g - b) / chroma).rem_euclid(6.0),
        m if m == g => (b - r) / chroma + 2.0,
        _ => (r - g) / chroma + 4.0,
    };
    (sector * 60.0, chroma)
}

// серый остаётся той же ступенью между фоном и текстом
fn neutral(color: u32, theme: &Theme) -> u32 {
    let [r, g, b] = channels(color);
    let light = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
    let t = ((light - 0.08) / 0.77).clamp(0.0, 1.0);
    let (from, to) = (channels(theme.background), channels(theme.foreground));
    let mixed = [0, 1, 2].map(|i| (from[i] + (to[i] - from[i]) * t).round() as u32);
    mixed[0] << 16 | mixed[1] << 8 | mixed[2]
}

/// Цвет темы, играющий ту же роль, что `color`.
fn matching(color: u32, theme: &Theme) -> u32 {
    let (angle, chroma) = hue(color);
    if chroma < NEUTRAL {
        return neutral(color, theme);
    }
    let distance = |other: u32| {
        let difference = (hue(other).0 - angle).abs();
        difference.min(360.0 - difference) as u32
    };
    let accents = theme.ansi[1..7].iter().copied();
    accents.min_by_key(|&c| distance(c)).unwrap_or(color)
}

/// Заменяет все `#rrggbb` в тексте цветами темы.
fn recolor(text: &str, theme: &Theme) -> String {
    let mut parts = text.split('#');
    let mut out = parts.next().unwrap_or_default().to_owned();
    for part in parts {
        let hex = part
            .get(..6)
            .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()));
        let color = hex.and_then(|h| u32::from_str_radix(h, 16).ok());
        match color {
            Some(color) => out.push_str(&format!("#{:06x}{}", matching(color, theme), &part[6..])),
            None => out.extend(["#", part]),
        }
    }
    out
}

// приглашение стиля или файл пользователя, при необходимости перекрашенные
fn wanted(config: &Config, paint: bool, template: Option<&str>) -> Option<String> {
    let text = match template {
        Some(template) => template.to_owned(),
        None => std::fs::read_to_string(source(config)?).ok()?,
    };
    Some(match paint {
        true => recolor(&text, &config.theme),
        false => text,
    })
}

fn write(config: &Config, paint: bool, template: Option<&str>) -> std::io::Result<()> {
    let Some(copy) = managed() else { return Ok(()) };
    let Some(text) = wanted(config, paint, template) else {
        // своего файла нет и стиль снят: убираем приглашение стиля
        return match std::fs::remove_file(&copy) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    };
    if std::fs::read_to_string(&copy).is_ok_and(|old| old == text) {
        return Ok(());
    }
    copy.parent().map_or(Ok(()), std::fs::create_dir_all)?;
    std::fs::write(&copy, text)
}

/// Обновляет копию: `template` — приглашение стиля, `paint` — перекрашивать.
pub(crate) fn sync(config: &Config, paint: bool, template: Option<&str>) {
    if let Err(e) = write(config, paint, template) {
        log::warn!("{}", t!("Приглашение не перекрашено: {error}", error = e));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colors_follow_the_theme_and_the_rest_stays() {
        let theme = Theme {
            background: 0x000000,
            foreground: 0xffffff,
            ansi: [
                0, 0xff0000, 0x00ff00, 0xffff00, 0x0000ff, 0xff00ff, 0x00ffff, 0, 0, 0, 0, 0, 0, 0,
                0, 0,
            ],
            ..Theme::default()
        };
        let out = recolor("# note\nfg:#f38ba8 bg:#11111b '#zz' #89b4fa", &theme);
        assert_eq!(out, "# note\nfg:#ff0000 bg:#000000 '#zz' #0000ff");
    }
}
