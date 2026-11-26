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
