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
    /// OSC 5: цвета жирного, подчёркнутого и прочего по номеру.
    pub special: BTreeMap<u8, u32>,
    /// OSC 13–19, кроме перечисленных выше: 17 — выделение, 19 — его текст.
    pub dynamic: BTreeMap<u16, u32>,
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

    // цвет из темы, который отвечают на запрос неизменённого
    fn theme_color(&self, code: u16) -> u32 {
        match code {
            11 | 14 | 16 | 19 => self.colors.background,
            12 => self.colors.cursor,
            _ => self.colors.foreground,
        }
    }

    fn dynamic_slot(&self, code: u16) -> Option<u32> {
        let overrides = &self.overrides;
        match code {
            10 => overrides.foreground,
            11 => overrides.background,
            12 => overrides.cursor,
            _ => overrides.dynamic.get(&code).copied(),
        }
    }

    fn set_dynamic(&mut self, code: u16, color: Option<u32>) {
        let overrides = &mut self.overrides;
        match code {
            10 => overrides.foreground = color,
            11 => overrides.background = color,
            12 => overrides.cursor = color,
            _ => match color {
                Some(color) => drop(overrides.dynamic.insert(code, color)),
                None => drop(overrides.dynamic.remove(&code)),
            },
        }
    }

    /// OSC 10–19: несколько значений подряд идут по следующим номерам.
    pub(crate) fn dynamic_colors(&mut self, first: u16, value: &str) {
        for (code, spec) in (first..=19).zip(value.split(';')) {
            if spec == "?" {
                let color = self
                    .dynamic_slot(code)
                    .unwrap_or_else(|| self.theme_color(code));
                self.report_rgb(&code.to_string(), color);
            } else if let Some(color) = parse(spec) {
                self.set_dynamic(code, Some(color));
            }
        }
    }

    /// OSC 5: пары «номер;цвет», 0 — жирный.
    pub(crate) fn special_colors(&mut self, value: &str) {
        let mut parts = value.split(';');
        while let (Some(index), Some(spec)) = (parts.next(), parts.next()) {
            let Some(index) = index.parse::<u8>().ok().filter(|i| *i < 5) else {
                continue;
            };
            if spec == "?" {
                let color = self.overrides.special.get(&index).copied();
                let color = color.unwrap_or(self.colors.foreground);
                self.report_rgb(&format!("5;{index}"), color);
            } else if let Some(color) = parse(spec) {
                self.overrides.special.insert(index, color);
            }
        }
    }

    /// OSC 104, 105 и 110–119: возврат к цветам темы.
    pub(crate) fn reset_colors(&mut self, code: u16, value: &str) {
        let listed = || value.split(';').filter_map(|i| i.parse::<u8>().ok());
        match code {
            104 if value.is_empty() => self.overrides.palette.clear(),
            104 => listed().for_each(|i| {
                let _ = self.overrides.palette.remove(&i);
            }),
            105 if value.is_empty() => self.overrides.special.clear(),
            105 => listed().for_each(|i| {
                let _ = self.overrides.special.remove(&i);
            }),
            _ => self.set_dynamic(code - 100, None),
        }
    }
}
