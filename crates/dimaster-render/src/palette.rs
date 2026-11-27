//! Перевод цветов сетки в RGB.

use dimaster_grid::Color;

/// RGB, упакованный как 0xRRGGBB.
pub type Rgb = u32;

pub(crate) fn resolve(color: Color, default: Rgb, ansi: &[Rgb; 16]) -> Rgb {
    match color {
        Color::Default => default,
        Color::Rgb(r, g, b) => rgb(r, g, b),
        Color::Indexed(index) => indexed(index, ansi),
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}

// 16 цветов темы, куб 6×6×6 и 24 ступени серого — стандарт xterm
fn indexed(index: u8, ansi: &[Rgb; 16]) -> Rgb {
    match index {
        0..=15 => ansi[usize::from(index)],
        16..=231 => {
            let n = index - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            rgb(level(n / 36), level(n / 6 % 6), level(n % 6))
        }
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            rgb(gray, gray, gray)
        }
    }
}

/// Приглушает цвет для атрибута DIM, смешивая с фоном.
pub(crate) fn dim(color: Rgb, background: Rgb) -> Rgb {
    let channel = |value: u32, shift: u32| (value >> shift) & 0xff;
    let mix = |shift| ((channel(color, shift) * 2 + channel(background, shift)) / 3) << shift;
    mix(16) | mix(8) | mix(0)
}

fn srgb_luminance(c: Rgb) -> f32 {
    let lin = |shift: u32| {
        let v = ((c >> shift) & 0xff) as f32 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(16) + 0.7152 * lin(8) + 0.0722 * lin(0)
}

fn contrast_ratio(a: Rgb, b: Rgb) -> f32 {
    let (l1, l2) = (srgb_luminance(a), srgb_luminance(b));
    let (hi, lo) = (l1.max(l2), l1.min(l2));
    (hi + 0.05) / (lo + 0.05)
}

/// Подтягивает цвет текста, если его контраст с фоном ниже `min_ratio`.
pub(crate) fn ensure_contrast(fg: Rgb, bg: Rgb, min_ratio: f32) -> Rgb {
    if min_ratio <= 1.0 || fg == bg || contrast_ratio(fg, bg) >= min_ratio {
        return fg;
    }
    let target =
        if srgb_luminance(bg) < dimaster_limits::LIGHT_BACKGROUND_PERCENT.get() as f32 / 100.0 {
            0xffffff
        } else {
            0x000000
        };
    if contrast_ratio(target, bg) < min_ratio {
        return target;
    }
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..8 {
        let mid = (lo + hi) * 0.5;
        let candidate = lerp_rgb(fg, target, mid);
        if contrast_ratio(candidate, bg) >= min_ratio {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    lerp_rgb(fg, target, hi)
}

fn lerp_rgb(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let ch = |shift: u32| {
        let (x, y) = (((a >> shift) & 0xff) as f32, ((b >> shift) & 0xff) as f32);
        ((x + (y - x) * t).round() as u32) << shift
    };
    ch(16) | ch(8) | ch(0)
}
