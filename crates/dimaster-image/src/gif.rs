//! Свой декодер GIF с анимацией, прозрачностью и чересстрочными кадрами.

use crate::{Animation, Frame};

const MAX_CODES: usize = 4096;

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let (head, tail) = self.0.split_at_checked(count)?;
        self.0 = tail;
        Some(head)
    }

    fn byte(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn word(&mut self) -> Option<usize> {
        let raw = self.take(2)?;
        Some(usize::from(u16::from_le_bytes([raw[0], raw[1]])))
    }

    // данные разбиты на блоки с длиной впереди, конец — блок длины 0
    fn blocks(&mut self) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        loop {
            match self.byte()? {
                0 => return Some(out),
                size => out.extend_from_slice(self.take(usize::from(size))?),
            }
        }
    }

    fn palette(&mut self, flags: u8) -> Option<&'a [u8]> {
        if flags & 0x80 == 0 {
            return Some(&[]);
        }
        self.take(3 << ((flags & 7) + 1))
    }
}

/// Параметры из расширения Graphic Control.
#[derive(Default, Clone, Copy)]
struct Control {
    delay: u32,
    transparent: Option<u8>,
    disposal: u8,
}

#[derive(Clone, Copy)]
struct Area {
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    interlaced: bool,
}

/// Словарь LZW: каждый код — это предыдущий код плюс один символ.
struct Lzw {
    prefix: Vec<u16>,
    suffix: Vec<u8>,
    minimum: u32,
}

impl Lzw {
    fn new(minimum: u32) -> Self {
        let clear = 1usize << minimum;
        let suffix = (0..clear + 2).map(|i| i as u8).collect();
        Self {
            prefix: vec![0; clear + 2],
            suffix,
            minimum,
        }
    }

    fn reset(&mut self) {
        let size = (1 << self.minimum) + 2;
        self.prefix.truncate(size);
        self.suffix.truncate(size);
    }

    // разворачивает код в конец `out` и возвращает первый символ строки
    fn expand(&self, mut code: usize, out: &mut Vec<u8>) -> u8 {
        let start = out.len();
        while code >= (1 << self.minimum) {
            out.push(self.suffix[code]);
            code = usize::from(self.prefix[code]);
        }
        out.push(code as u8);
        out[start..].reverse();
        out[start]
    }

    fn add(&mut self, prefix: usize, symbol: u8) {
        if self.prefix.len() < MAX_CODES {
            self.prefix.push(prefix as u16);
            self.suffix.push(symbol);
        }
    }

    // следующий код словаря; он может ссылаться сам на себя
    fn step(&mut self, code: usize, previous: Option<usize>, out: &mut Vec<u8>) -> Option<()> {
        let size = self.prefix.len();
        match previous {
            Some(previous) if code == size => {
                let mut probe = Vec::new();
                let first = self.expand(previous, &mut probe);
                self.add(previous, first);
                self.expand(code.min(self.prefix.len() - 1), out);
            }
            _ if code < size => {
                let first = self.expand(code, out);
                if let Some(previous) = previous {
                    self.add(previous, first);
                }
            }
            _ => return None,
        }
        Some(())
    }
}

fn unpack(data: &[u8], minimum: u32, pixels: usize) -> Option<Vec<u8>> {
    if !(2..=8).contains(&minimum) {
        return None;
    }
    let (mut table, mut out) = (Lzw::new(minimum), Vec::with_capacity(pixels));
    let clear = 1usize << minimum;
    let (mut position, mut width, mut previous) = (0, minimum + 1, None);
    while out.len() < pixels && position + width as usize <= data.len() * 8 {
        let code = read_code(data, position, width);
        position += width as usize;
        if code == clear {
            table.reset();
            (width, previous) = (minimum + 1, None);
        } else if code == clear + 1 {
            break;
        } else {
            table.step(code, previous, &mut out)?;
            previous = Some(code);
            if table.prefix.len() == 1 << width && width < 12 {
                width += 1;
            }
        }
    }
    out.resize(pixels, 0);
    Some(out)
}

fn read_code(data: &[u8], position: usize, width: u32) -> usize {
    (0..width as usize).fold(0, |code, i| {
        let bit = (data[(position + i) / 8] >> ((position + i) % 8)) & 1;
        code | usize::from(bit) << i
    })
}

// чересстрочный кадр идёт четырьмя проходами
fn row_order(height: usize, interlaced: bool) -> Vec<usize> {
    if !interlaced {
        return (0..height).collect();
    }
    [(0, 8), (4, 8), (2, 4), (1, 2)]
        .iter()
        .flat_map(|&(start, step)| (start..height).step_by(step))
        .collect()
}

fn draw(
    canvas: &mut [u8],
    width: usize,
    area: Area,
    indices: &[u8],
    palette: &[u8],
    skip: Option<u8>,
) {
    for (line, y) in row_order(area.height, area.interlaced)
        .into_iter()
        .enumerate()
    {
        let row = &indices[line * area.width..][..area.width];
        for (x, &index) in row.iter().enumerate() {
            let Some(rgb) = palette.get(usize::from(index) * 3..usize::from(index) * 3 + 3) else {
                continue;
            };
            if Some(index) != skip {
                let at = ((area.top + y) * width + area.left + x) * 4;
                canvas[at..at + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
    }
}

struct Decoder<'a> {
    reader: Reader<'a>,
    global: &'a [u8],
    canvas: Vec<u8>,
    animation: Animation,
    control: Control,
}

impl<'a> Decoder<'a> {
    fn open(data: &'a [u8]) -> Option<Self> {
        let mut reader = Reader(data);
        let signature = reader.take(6)?;
        let (width, height) = (reader.word()?, reader.word()?);
        let flags = reader.byte()?;
        reader.take(2)?;
        let sizes = (1..=dimaster_limits::GIF_SIDE.size()).contains(&width)
            && (1..=dimaster_limits::GIF_SIDE.size()).contains(&height);
        if !signature.starts_with(b"GIF8") || !sizes {
            return None;
        }
        let global = reader.palette(flags)?;
        let canvas = vec![0; width * height * 4];
        let animation = Animation {
            width,
            height,
            frames: Vec::new(),
        };
        Some(Self {
            reader,
            global,
            canvas,
            animation,
            control: Control::default(),
        })
    }

    fn extension(&mut self) -> Option<()> {
        let label = self.reader.byte()?;
        let data = self.reader.blocks()?;
        if label == 0xf9 && data.len() >= 4 {
            let delay = u32::from(u16::from_le_bytes([data[1], data[2]])) * 10;
            self.control = Control {
                // браузеры показывают «нулевую» задержку как 100 мс
                delay: if delay < 20 { 100 } else { delay },
                transparent: (data[0] & 1 != 0).then_some(data[3]),
                disposal: (data[0] >> 2) & 7,
            };
        }
        Some(())
    }

    fn area(&mut self) -> Option<(Area, &'a [u8])> {
        let (left, top) = (self.reader.word()?, self.reader.word()?);
        let (width, height) = (self.reader.word()?, self.reader.word()?);
        let flags = self.reader.byte()?;
        let fits = left + width <= self.animation.width && top + height <= self.animation.height;
        let local = self.reader.palette(flags)?;
        let palette = if local.is_empty() { self.global } else { local };
        let area = Area {
            left,
            top,
            width,
            height,
            interlaced: flags & 0x40 != 0,
        };
        fits.then_some((area, palette))
    }

    fn image(&mut self) -> Option<()> {
        let (area, palette) = self.area()?;
        let minimum = u32::from(self.reader.byte()?);
        let indices = unpack(&self.reader.blocks()?, minimum, area.width * area.height)?;
        let control = std::mem::take(&mut self.control);
        let before = (control.disposal == 3).then(|| self.canvas.clone());
        draw(
            &mut self.canvas,
            self.animation.width,
            area,
            &indices,
            palette,
            control.transparent,
        );
        self.animation.frames.push(Frame {
            pixels: self.canvas.clone(),
            delay: control.delay,
        });
        match before {
            Some(canvas) => self.canvas = canvas,
            None if control.disposal == 2 => self.clear(area),
            None => {}
        }
        Some(())
    }

    fn clear(&mut self, area: Area) {
        for y in area.top..area.top + area.height {
            let start = (y * self.animation.width + area.left) * 4;
            self.canvas[start..start + area.width * 4].fill(0);
        }
    }

    // повреждённый хвост не отбрасывает уже прочитанные кадры
    fn run(mut self) -> Option<Animation> {
        while self.animation.bytes() + self.canvas.len() <= dimaster_limits::GIF_BYTES.size() {
            let step = match self.reader.byte() {
                Some(0x21) => self.extension(),
                Some(0x2c) => self.image(),
                _ => None,
            };
            if step.is_none() {
                break;
            }
        }
        (!self.animation.frames.is_empty()).then_some(self.animation)
    }
}

pub fn decode_gif(data: &[u8]) -> Option<Animation> {
    Decoder::open(data)?.run()
}
