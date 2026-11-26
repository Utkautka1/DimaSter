//! Цвета по умолчанию; настройка тем появится вместе с Lua-конфигом.

use dimaster_grid::Color;

/// RGB, упакованный как 0xRRGGBB.
pub(crate) type Rgb = u32;

pub(crate) const FOREGROUND: Rgb = 0xd8dee9;
pub(crate) const BACKGROUND: Rgb = 0x11131a;
pub(crate) const SELECTION: Rgb = 0x3b4261;
pub(crate) const CURSOR: Rgb = 0xe5e9f0;

const ANSI: [Rgb; 16] = [
    0x1d1f28, 0xf7768e, 0x9ece6a, 0xe0af68, 0x7aa2f7, 0xbb9af7, 0x7dcfff, 0xc0caf5, 0x565f89,
    0xff899d, 0xb9f27c, 0xffc777, 0x8db0ff, 0xc7a9ff, 0xa4daff, 0xe5e9f0,
];

pub(crate) fn resolve(color: Color, default: Rgb) -> Rgb {
    match color {
        Color::Default => default,
        Color::Rgb(r, g, b) => rgb(r, g, b),
        Color::Indexed(index) => indexed(index),
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Rgb {
    u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b)
}

// 16 цветов темы, куб 6×6×6 и 24 ступени серого — стандарт xterm
fn indexed(index: u8) -> Rgb {
    match index {
        0..=15 => ANSI[usize::from(index)],
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
