//! Фоны стилей, нарисованные кодом из цветов палитры: без файлов и персонажей.

use dimaster_config::{Scene, Theme};
use dimaster_image::Animation;

const WIDTH: usize = 960;
const HEIGHT: usize = 600;

type Rgb = [f32; 3];

fn rgb(color: u32) -> Rgb {
    [16, 8, 0].map(|shift| ((color >> shift) & 0xff) as f32)
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

// псевдослучайное число 0..1: одна и та же картинка при каждом запуске
fn noise(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9e37_79b9) ^ 0x5bd1_e995;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2c1b_3c6d);
    ((x ^ (x >> 12)) % 10_000) as f32 / 10_000.0
}

// расстояние в долях высоты: круг остаётся кругом
fn distance(x: f32, y: f32, cx: f32, cy: f32) -> f32 {
    ((x - cx) * WIDTH as f32 / HEIGHT as f32).hypot(y - cy)
}

fn blobs(x: f32, y: f32, theme: &Theme) -> Rgb {
    let spots = [
        (0.15, 0.2, 5),
        (0.85, 0.3, 4),
        (0.4, 0.95, 6),
        (0.75, 0.85, 2),
    ];
    spots
        .iter()
        .fold(rgb(theme.background), |color, &(cx, cy, ansi)| {
            let power = 1.0 - distance(x, y, cx, cy) / 0.6;
            mix(color, rgb(theme.ansi[ansi]), power.max(0.0).powi(2))
        })
}

// неоновая сетка до горизонта и полосатое солнце
fn grid(x: f32, y: f32, theme: &Theme) -> Rgb {
    let sky = mix(rgb(theme.background), rgb(theme.ansi[5]), y * 0.9);
    if y < 0.62 {
        let sun = distance(x, y, 0.5, 0.5) < 0.22 && (y < 0.46 || (y * 60.0).fract() > 0.35);
        return if sun {
            mix(rgb(theme.ansi[3]), rgb(theme.ansi[1]), (y - 0.28) * 3.0)
        } else {
            sky
        };
    }
    let depth = (y - 0.62) / 0.38;
    let across = (((x - 0.5) / (depth + 0.08)) * 6.0).fract().abs();
    let along = (1.0 / (depth + 0.12) * 2.0).fract();
    let line = !(0.03..=0.97).contains(&across) || along < 0.07;
    let floor = rgb(theme.background);
    if line { rgb(theme.ansi[6]) } else { floor }
}

fn scanlines(x: f32, y: f32, theme: &Theme) -> Rgb {
    let glow = 1.0 - distance(x, y, 0.5, 0.5);
    let base = mix(rgb(theme.background), rgb(theme.ansi[2]), glow * 0.35);
    let dark = (y * HEIGHT as f32) as usize % 4 < 2;
    mix(base, [0.0; 3], if dark { 0.45 } else { 0.0 })
}

fn waves(x: f32, y: f32, theme: &Theme) -> Rgb {
    let sky = mix(rgb(theme.ansi[4]), rgb(theme.ansi[5]), x);
    let mut color = mix(rgb(theme.background), sky, 0.35 + y * 0.3);
    for (i, ansi) in [4, 6, 5].into_iter().enumerate() {
        let level = 0.55 + i as f32 * 0.14 + (x * 7.0 + i as f32 * 2.1).sin() * 0.06;
        if y > level {
            color = mix(color, rgb(theme.ansi[ansi]), 0.4);
        }
    }
    color
}

// вечернее небо, солнце и две гряды гор
fn sunset(x: f32, y: f32, theme: &Theme) -> Rgb {
    let far = 0.66 + (x * 9.0).sin() * 0.05 + (x * 23.0).sin() * 0.025;
    let near = 0.82 + (x * 6.0 + 1.7).sin() * 0.07;
    if y > near {
        return rgb(theme.background);
    }
    if y > far {
        return mix(rgb(theme.background), rgb(theme.ansi[5]), 0.35);
    }
    let sky = mix(rgb(theme.ansi[4]), rgb(theme.ansi[1]), y * 1.5);
    let sun = distance(x, y, 0.68, 0.5) < 0.13;
    if sun { rgb(theme.ansi[11]) } else { sky }
}

// дома разной высоты, окна и звёзды
fn city(x: f32, y: f32, theme: &Theme) -> Rgb {
    let block = (x * 28.0) as u32;
    let roof = 0.45 + noise(block) * 0.38;
    let (px, py) = ((x * WIDTH as f32) as u32, (y * HEIGHT as f32) as u32);
    if y > roof {
        let lit = px % 9 > 4 && py % 14 > 8 && noise(px / 9 * 977 + py / 14) > 0.68;
        let wall = mix(rgb(theme.background), [0.0; 3], 0.35);
        return if lit { rgb(theme.ansi[11]) } else { wall };
    }
    let star = noise(px / 3 * 7919 + py / 3) > 0.9975;
    let sky = mix(rgb(theme.background), rgb(theme.ansi[4]), y * 0.7);
    if star { rgb(theme.foreground) } else { sky }
}

// ветер несёт лепестки: наклонные эллипсы в случайных местах
fn petals(x: f32, y: f32, theme: &Theme) -> Rgb {
    let sky = mix(rgb(theme.background), rgb(theme.ansi[5]), y * 0.35);
    let hit = (0..90u32).any(|i| {
        let (cx, cy) = (noise(i * 3), noise(i * 3 + 1));
        let size = 0.008 + noise(i * 3 + 2) * 0.016;
        let (dx, dy) = ((x - cx) * 1.6, y - cy);
        let (u, v) = (dx * 0.8 + dy * 0.6, dy * 0.8 - dx * 0.6);
        (u / size).powi(2) + (v / (size * 0.55)).powi(2) < 1.0
    });
    if hit {
        mix(rgb(theme.ansi[13]), [255.0; 3], 0.25)
    } else {
        sky
    }
}

fn painter(scene: Scene) -> Option<fn(f32, f32, &Theme) -> Rgb> {
    Some(match scene {
        Scene::None => return None,
        Scene::Blobs => blobs,
        Scene::Grid => grid,
        Scene::Scanlines => scanlines,
        Scene::Waves => waves,
        Scene::Sunset => sunset,
        Scene::City => city,
        Scene::Petals => petals,
    })
}

/// Картинка фона; `None` — у сцены его нет.
pub(crate) fn paint(scene: Scene, theme: &Theme) -> Option<Animation> {
    let draw = painter(scene)?;
    let mut pixels = Vec::with_capacity(WIDTH * HEIGHT * 4);
    for row in 0..HEIGHT {
        for column in 0..WIDTH {
            let x = column as f32 / WIDTH as f32;
            let color = draw(x, row as f32 / HEIGHT as f32, theme);
            pixels.extend(color.map(|c| c.round() as u8));
            pixels.push(255);
        }
    }
    Some(Animation::still(WIDTH, HEIGHT, pixels))
}

#[cfg(test)]
mod tests {
    use dimaster_config::styles;

    use super::*;

    #[test]
    fn every_scene_fills_the_whole_picture() {
        let theme = Theme::default();
        for style in styles() {
            match paint(style.scene, &theme) {
                Some(picture) => assert_eq!(picture.frames[0].pixels.len(), WIDTH * HEIGHT * 4),
                None => assert_eq!(style.scene, Scene::None),
            }
        }
    }
}
