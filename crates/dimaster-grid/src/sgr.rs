//! SGR (`CSI ... m`): цвета и атрибуты текста.

use dimaster_vt::Params;

use crate::{Color, Flags, Style};

/// Применяет все параметры SGR к стилю; пустой список означает сброс.
pub(crate) fn apply(style: &mut Style, params: &Params) {
    let mut groups = params.groups().peekable();
    if groups.peek().is_none() {
        *style = Style::default();
    }
    while let Some(group) = groups.next() {
        match group {
            [target @ (38 | 48)] => {
                // форма через `;`: цвет занимает следующие параметры
                let mut rest = std::iter::from_fn(|| groups.next().map(|g| g[0]));
                let color = read_color(&mut rest);
                set_color(style, *target, color);
            }
            [target @ (38 | 48), sub @ ..] => set_color(style, *target, colon_color(sub)),
            // 4:0 выключает подчёркивание, 4:1..5 — разные его виды
            [4, kind] => style.flags.set(Flags::UNDERLINE, *kind != 0),
            [code, ..] => apply_code(style, *code),
            [] => {}
        }
    }
}

fn set_color(style: &mut Style, target: u16, color: Option<Color>) {
    let Some(color) = color else { return };
    if target == 38 {
        style.fg = color;
    } else {
        style.bg = color;
    }
}

// В форме 38:2:<colorspace>:r:g:b лишний идентификатор надо пропустить
fn colon_color(sub: &[u16]) -> Option<Color> {
    match sub {
        [2, _, r, g, b, ..] => read_color(&mut [2, *r, *g, *b].into_iter()),
        _ => read_color(&mut sub.iter().copied()),
    }
}

fn read_color(values: &mut impl Iterator<Item = u16>) -> Option<Color> {
    let mut byte = || values.next().and_then(|v| u8::try_from(v).ok());
    match byte()? {
        5 => Some(Color::Indexed(byte()?)),
        2 => Some(Color::Rgb(byte()?, byte()?, byte()?)),
        _ => None,
    }
}

fn apply_code(style: &mut Style, code: u16) {
    // код укладывается в u8 во всех ветках с индексами палитры
    let index = |base: u16, offset: u8| Color::Indexed((code - base) as u8 + offset);
    match code {
        0 => *style = Style::default(),
        1..=9 | 21..=29 => apply_flag(style, code),
        30..=37 => style.fg = index(30, 0),
        39 => style.fg = Color::Default,
        40..=47 => style.bg = index(40, 0),
        49 => style.bg = Color::Default,
        90..=97 => style.fg = index(90, 8),
        100..=107 => style.bg = index(100, 8),
        _ => {}
    }
}

fn apply_flag(style: &mut Style, code: u16) {
    let on = code < 20;
    let flag = match code % 20 {
        1 if on => Flags::BOLD,
        // 21 — двойное подчёркивание, 22 снимает и bold, и dim
        1 => Flags::UNDERLINE,
        2 if on => Flags::DIM,
        2 => {
            style.flags.set(Flags::BOLD, false);
            Flags::DIM
        }
        3 => Flags::ITALIC,
        4 => Flags::UNDERLINE,
        5 | 6 => Flags::BLINK,
        7 => Flags::INVERSE,
        8 => Flags::HIDDEN,
        9 => Flags::STRIKE,
        _ => return,
    };
    style.flags.set(flag, on || code == 21);
}
