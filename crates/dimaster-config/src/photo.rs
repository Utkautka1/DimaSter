//! Палитра по картинке: фон и текст берут её общий тон, акцент — самый
//! насыщенный цвет. Остальные цвета ANSI держатся своих оттенков (красный
//! остаётся красным), чтобы вывод программ читался на любом фоне.

use crate::Theme;

// оттенки red, green, yellow, blue, magenta, cyan в градусах
const HUES: [f32; 6] = [355.0, 125.0, 45.0, 220.0, 305.0, 185.0];

/// Сумма цветов с весами; из неё получается средний оттенок.
#[derive(Default, Clone, Copy)]
struct Mix {
    sum: [f32; 3],
    weight: f32,
}

impl Mix {
    fn add(&mut self, rgb: [f32; 3], weight: f32) {
        for (total, value) in self.sum.iter_mut().zip(rgb) {
            *total += value * weight;
        }
        self.weight += weight;
    }

    fn color(self) -> [f32; 3] {
        self.sum.map(|v| v / self.weight.max(f32::EPSILON))
    }
}

fn hue(rgb: [f32; 3]) -> f32 {
    let [r, g, b] = rgb;
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let span = (max - min).max(f32::EPSILON);
    let sector = if max == r {
        ((g - b) / span).rem_euclid(6.0)
    } else if max == g {
        (b - r) / span + 2.0
    } else {
        (r - g) / span + 4.0
    };
    sector * 60.0
}

fn hsl(hue: f32, saturation: f32, light: f32) -> u32 {
    let amount = saturation * light.min(1.0 - light);
    let channel = |n: f32| {
        let k = (n + hue.rem_euclid(360.0) / 30.0) % 12.0;
        let value = light - amount * (k - 3.0).min(9.0 - k).clamp(-1.0, 1.0);
        (value * 255.0).round() as u32
    };
    channel(0.0) << 16 | channel(8.0) << 8 | channel(4.0)
}

// общий тон, акцент по насыщенным пикселям и средняя яркость
fn measure(rgba: &[u8]) -> (f32, f32, f32) {
    let (mut all, mut vivid, mut light) = (Mix::default(), Mix::default(), 0.0);
    let step = (rgba.len() / 4 / dimaster_limits::PHOTO_SAMPLES.size()).max(1);
    for pixel in rgba.as_chunks::<4>().0.iter().step_by(step) {
        let rgb = [pixel[0], pixel[1], pixel[2]].map(|v| f32::from(v) / 255.0);
        let max = rgb[0].max(rgb[1]).max(rgb[2]);
        let min = rgb[0].min(rgb[1]).min(rgb[2]);
        all.add(rgb, 1.0);
        vivid.add(rgb, (max - min).powi(3) * max);
        light += 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
    }
    let accent = if vivid.weight > 0.0 { vivid } else { all };
    let light = light / all.weight.max(1.0);
    (hue(all.color()), hue(accent.color()), light)
}

// цвет оттенка `base`, слегка смещённый к акценту картинки
fn toward(base: f32, accent: f32, light: f32) -> u32 {
    let turn = (accent - base + 540.0) % 360.0 - 180.0;
    hsl(base + turn * 0.12, 0.68, light)
}

fn ansi(tone: f32, accent: f32) -> [u32; 16] {
    let mut colors = [0; 16];
    for (half, light) in [(0, 0.70), (8, 0.78)] {
        for (i, &base) in HUES.iter().enumerate() {
            colors[half + 1 + i] = toward(base, accent, light);
        }
        // «синий» — цвет акцента: им рисуются каталоги и строки статуса
        colors[half + 4] = hsl(accent, 0.72, light);
    }
    colors[0] = hsl(tone, 0.22, 0.26);
    colors[7] = hsl(tone, 0.22, 0.78);
    colors[8] = hsl(tone, 0.18, 0.45);
    colors[15] = hsl(tone, 0.25, 0.92);
    colors
}

/// Тёмная палитра под картинку RGBA и её яркость от 0 до 1.
pub fn photo_theme(rgba: &[u8]) -> (Theme, f32) {
    let (tone, accent, light) = measure(rgba);
    let foreground = hsl(tone, 0.25, 0.90);
    let theme = Theme {
        foreground,
        background: hsl(tone, 0.30, 0.08),
        cursor: hsl(accent, 0.80, 0.70),
        selection: hsl(accent, 0.50, 0.34),
        selection_text: Some(foreground),
        ansi: ansi(tone, accent),
    };
    (theme, light)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(color: [u8; 4]) -> Vec<u8> {
        color.repeat(64)
    }

    #[test]
    fn accent_follows_the_picture() {
        let (theme, light) = photo_theme(&picture([20, 40, 220, 255]));
        let blue = theme.cursor & 0xff;
        assert!(blue > theme.cursor >> 16);
        assert!(light < 0.2);
    }

    #[test]
    fn text_stays_readable_on_white_pictures() {
        let (theme, light) = photo_theme(&picture([255; 4]));
        assert!(light > 0.9);
        assert!(theme.background >> 16 < 0x30);
        assert!(theme.foreground >> 16 > 0xd0);
    }
}
