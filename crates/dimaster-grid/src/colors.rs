//! Цвета, которые меняют сами программы (OSC 4, 10–12 и их сброс).

use std::collections::BTreeMap;

use crate::terminal::State;

/// Цвета темы как 0xRRGGBB — их сообщают в ответ на запросы.
#[derive(Debug, Clone, Copy)]
pub struct DefaultColors {
    pub foreground: u32,
    pub background: u32,
    pub cursor: u32,
    pub ansi: [u32; 16],
}

impl Default for DefaultColors {
    fn default() -> Self {
        Self {
            foreground: 0x00d8_dee9,
            background: 0x0011_131a,
            cursor: 0x00e5_e9f0,
            ansi: [0; 16],
        }
    }
}

/// Изменения поверх темы; пусто, пока программа ничего не просила.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ColorOverrides {
    pub palette: BTreeMap<u8, u32>,
    pub foreground: Option<u32>,
    pub background: Option<u32>,
    pub cursor: Option<u32>,
}

// `#rgb`, `#rrggbb` или `rgb:r/g/b` с 1–4 цифрами на канал
fn parse(spec: &str) -> Option<u32> {
    if let Some(hex) = spec.strip_prefix('#') {
        let wide = |h: &str| u32::from_str_radix(h, 16).ok();
        return match (hex.len(), hex.is_ascii()) {
            (6, true) => wide(hex),
            (3, true) => wide(&hex.chars().flat_map(|c| [c, c]).collect::<String>()),
            _ => None,
        };
    }
    let channels: Vec<u32> = spec
        .strip_prefix("rgb:")?
        .split('/')
        .map_while(channel)
        .collect();
    match channels[..] {
        [r, g, b] => Some(r << 16 | g << 8 | b),
        _ => None,
    }
}

// приводит канал любой точности к 8 битам
fn channel(hex: &str) -> Option<u32> {
    let digits = u32::try_from(hex.len())
        .ok()
        .filter(|d| (1..=4).contains(d))?;
    let value = u32::from_str_radix(hex, 16).ok()?;
    Some(value * 255 / ((1 << (digits * 4)) - 1))
}

// вторая половина палитры по стандарту xterm: куб 6×6×6 и оттенки серого
fn xterm(index: u8) -> u32 {
    if index >= 232 {
        let gray = u32::from(8 + (index - 232) * 10);
        return gray << 16 | gray << 8 | gray;
    }
    let level = |v: u8| if v == 0 { 0 } else { u32::from(55 + v * 40) };
    let n = index - 16;
    level(n / 36) << 16 | level(n / 6 % 6) << 8 | level(n % 6)
}

impl State {
    fn report_rgb(&mut self, code: &str, color: u32) {
        let [_, r, g, b] = color.to_be_bytes().map(|c| u16::from(c) * 0x101);
        self.answer(format!("\x1b]{code};rgb:{r:04x}/{g:04x}/{b:04x}\x1b\\"));
    }

    /// OSC 4: пары «номер;цвет», `?` вместо цвета — запрос.
    pub(crate) fn palette_osc(&mut self, value: &str) {
        let mut parts = value.split(';');
        while let (Some(index), Some(spec)) = (parts.next(), parts.next()) {
            let Ok(index) = index.parse::<u8>() else {
                continue;
            };
            if spec == "?" {
                let theme = self.colors.ansi.get(usize::from(index)).copied();
                let fallback = theme.unwrap_or_else(|| xterm(index.max(16)));
                let color = self
                    .overrides
                    .palette
                    .get(&index)
                    .copied()
                    .unwrap_or(fallback);
                self.report_rgb(&format!("4;{index}"), color);
            } else if let Some(color) = parse(spec) {
                self.overrides.palette.insert(index, color);
            }
        }
    }

    /// OSC 10, 11, 12: цвет текста, фона и курсора.
    pub(crate) fn dynamic_color(&mut self, code: &str, value: &str) {
        let (theme, slot) = match code {
            "10" => (self.colors.foreground, &mut self.overrides.foreground),
            "11" => (self.colors.background, &mut self.overrides.background),
            _ => (self.colors.cursor, &mut self.overrides.cursor),
        };
        if value == "?" {
            let color = slot.unwrap_or(theme);
            self.report_rgb(code, color);
        } else if let Some(color) = parse(value) {
            *slot = Some(color);
        }
    }

    /// OSC 104 и 110–112: возврат к цветам темы.
    pub(crate) fn reset_colors(&mut self, code: &str, value: &str) {
        match code {
            "104" if value.is_empty() => self.overrides.palette.clear(),
            "104" => value
                .split(';')
                .filter_map(|i| i.parse().ok())
                .for_each(|i: u8| {
                    self.overrides.palette.remove(&i);
                }),
            "110" => self.overrides.foreground = None,
            "111" => self.overrides.background = None,
            _ => self.overrides.cursor = None,
        }
    }
}
