//! Декодер Sixel: каждый символ — столбик из шести пикселей одного цвета.

use std::collections::BTreeMap;

use crate::Image;

// первые 16 регистров VT340
const DEFAULT_COLORS: [u32; 16] = [
    0x000000, 0x3333cc, 0xcc2121, 0x33cc33, 0xcc33cc, 0x33cccc, 0xcccc33, 0x878787, 0x424242,
    0x545499, 0x994242, 0x549954, 0x995499, 0x549999, 0x999954, 0xcccccc,
];

#[derive(Default)]
struct Canvas {
    // разреженно: размер заранее неизвестен
    pixels: BTreeMap<(usize, usize), u32>,
    colors: BTreeMap<u32, u32>,
    color: u32,
    x: usize,
    band: usize,
    size: (usize, usize),
    painted: usize,
}

fn number(data: &mut &[u8]) -> u32 {
    let mut value = 0u32;
    while let Some((&digit @ b'0'..=b'9', rest)) = data.split_first() {
        value = value
            .saturating_mul(10)
            .saturating_add(u32::from(digit - b'0'));
        *data = rest;
    }
    value
}

fn numbers(data: &mut &[u8]) -> Vec<u32> {
    let mut out = vec![number(data)];
    while let Some((b';', rest)) = data.split_first() {
        *data = rest;
        out.push(number(data));
    }
    out
}

fn percent(value: u32) -> u32 {
    value.min(100) * 255 / 100
}

// цвет в HLS по соглашению DEC: тон 0 — синий
fn from_hls(hue: u32, lightness: u32, saturation: u32) -> u32 {
    let h = ((hue + 240) % 360) as f32 / 60.0;
    let (l, s) = (
        lightness.min(100) as f32 / 100.0,
        saturation.min(100) as f32 / 100.0,
    );
    let chroma = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let second = chroma * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (chroma, second, 0.0),
        1 => (second, chroma, 0.0),
        2 => (0.0, chroma, second),
        3 => (0.0, second, chroma),
        4 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let channel = |v: f32| ((v + l - chroma / 2.0) * 255.0).round() as u32;
    channel(r) << 16 | channel(g) << 8 | channel(b)
}

impl Canvas {
    // `#n` выбирает регистр, `#n;режим;a;b;c` ещё и задаёт его цвет
    fn select(&mut self, values: &[u32]) {
        self.color = values[0];
        if let [register, mode, a, b, c] = *values {
            let rgb = match mode {
                1 => from_hls(a, b, c),
                _ => percent(a) << 16 | percent(b) << 8 | percent(c),
            };
            self.colors.insert(register, rgb);
        }
    }

    fn rgb(&self) -> u32 {
        let fallback = DEFAULT_COLORS[(self.color % 16) as usize];
        self.colors.get(&self.color).copied().unwrap_or(fallback)
    }

    fn paint(&mut self, sixel: u8, repeat: usize) {
        let bits = sixel - b'?';
        let end = (self.x + repeat).min(dimaster_limits::SIXEL_SIDE.size());
        let top = self.band * 6;
        if top + 6 <= dimaster_limits::SIXEL_SIDE.size() {
            let rgb = self.rgb();
            self.painted += (end - self.x) * bits.count_ones() as usize;
            for row in (0..6).filter(|row| bits & (1 << row) != 0) {
                for x in self.x..end {
                    self.pixels.insert((top + row, x), rgb);
                }
                self.size = (self.size.0.max(end), self.size.1.max(top + row + 1));
            }
        }
        self.x = end;
    }

    fn command(&mut self, byte: u8, data: &mut &[u8]) {
        match byte {
            b'#' => self.select(&numbers(data)),
            b'!' => {
                let repeat =
                    number(data).clamp(1, dimaster_limits::SIXEL_SIDE.size() as u32) as usize;
                if let Some((&sixel @ b'?'..=b'~', rest)) = data.split_first() {
                    *data = rest;
                    self.paint(sixel, repeat);
                }
            }
            // растровые атрибуты задают минимальный размер картинки
            b'"' => {
                if let [_, _, width, height] = numbers(data)[..] {
                    let clamp = |v: u32| (v as usize).min(dimaster_limits::SIXEL_SIDE.size());
                    self.size = (
                        self.size.0.max(clamp(width)),
                        self.size.1.max(clamp(height)),
                    );
                }
            }
            b'$' => self.x = 0,
            b'-' => (self.x, self.band) = (0, self.band + 1),
            b'?'..=b'~' => self.paint(byte, 1),
            _ => {}
        }
    }

    fn finish(self) -> Option<Image> {
        let (width, height) = self.size;
        if width == 0 || height == 0 {
            return None;
        }
        let mut pixels = vec![0; width * height * 4];
        for ((y, x), rgb) in self.pixels {
            if x < width && y < height {
                let [_, r, g, b] = rgb.to_be_bytes();
                pixels[(y * width + x) * 4..][..4].copy_from_slice(&[r, g, b, 255]);
            }
        }
        Some(Image {
            width,
            height,
            pixels,
        })
    }
}

/// `data` — тело DCS после буквы `q`. Незакрашенные пиксели прозрачны.
pub fn decode_sixel(mut data: &[u8]) -> Option<Image> {
    // ограничение объёма защищает от повторов, раздувающих карту пикселей
    let mut canvas = Canvas::default();
    while let Some((&byte, rest)) = data.split_first() {
        data = rest;
        canvas.command(byte, &mut data);
        if canvas.pixels.len() > dimaster_limits::SIXEL_PIXELS.size()
            || canvas.painted > dimaster_limits::SIXEL_PAINT_BUDGET.size()
        {
            break;
        }
    }
    canvas.finish()
}
