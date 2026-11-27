//! Декодер PNG в RGBA: 8 бит на канал и палитры 1–8 бит, без интерлейса.

use crate::inflate;

/// Картинка в RGBA без предумножения альфы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

#[derive(Default)]
struct Chunks {
    header: Vec<u8>,
    palette: Vec<u8>,
    alpha: Vec<u8>,
    data: Vec<u8>,
}

struct Header {
    width: usize,
    height: usize,
    depth: usize,
    color: u8,
}

impl Header {
    fn parse(raw: &[u8]) -> Option<Self> {
        let side = |at: usize| Some(u32::from_be_bytes(raw.get(at..at + 4)?.try_into().ok()?));
        let (width, height) = (side(0)? as usize, side(4)? as usize);
        let (depth, color, interlace) = (usize::from(*raw.get(8)?), *raw.get(9)?, *raw.get(12)?);
        let sizes = (1..=dimaster_limits::PNG_SIDE.size()).contains(&width)
            && (1..=dimaster_limits::PNG_SIDE.size()).contains(&height);
        let depth_ok = depth == 8 || color == 3 && matches!(depth, 1 | 2 | 4);
        (sizes && depth_ok && interlace == 0).then_some(Self {
            width,
            height,
            depth,
            color,
        })
    }

    fn channels(&self) -> Option<usize> {
        match self.color {
            0 | 3 => Some(1),
            2 => Some(3),
            4 => Some(2),
            6 => Some(4),
            _ => None,
        }
    }
}

fn chunks(mut data: &[u8]) -> Option<Chunks> {
    data = data.strip_prefix(b"\x89PNG\r\n\x1a\n")?;
    let mut found = Chunks::default();
    while data.len() >= 12 {
        let length = u32::from_be_bytes(data[..4].try_into().ok()?) as usize;
        let body = data.get(8..8usize.checked_add(length)?)?;
        match &data[4..8] {
            b"IHDR" => found.header = body.to_vec(),
            b"PLTE" => found.palette = body.to_vec(),
            b"tRNS" => found.alpha = body.to_vec(),
            b"IDAT" => found.data.extend_from_slice(body),
            b"IEND" => return Some(found),
            _ => {}
        }
        data = data.get(12 + length..)?;
    }
    None
}

fn paeth(left: u8, up: u8, corner: u8) -> u8 {
    let estimate = i16::from(left) + i16::from(up) - i16::from(corner);
    let distance = |v: u8| (estimate - i16::from(v)).abs();
    if distance(left) <= distance(up) && distance(left) <= distance(corner) {
        left
    } else if distance(up) <= distance(corner) {
        up
    } else {
        corner
    }
}

// снимает построчный фильтр; `step` — расстояние до соседнего пикселя в байтах
fn unfilter(filter: u8, line: &mut [u8], previous: &[u8], step: usize) -> Option<()> {
    for i in 0..line.len() {
        let left = if i >= step { line[i - step] } else { 0 };
        let corner = if i >= step { previous[i - step] } else { 0 };
        let up = previous[i];
        let predicted = match filter {
            0 => 0,
            1 => left,
            2 => up,
            3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
            4 => paeth(left, up, corner),
            _ => return None,
        };
        line[i] = line[i].wrapping_add(predicted);
    }
    Some(())
}

fn scanlines(header: &Header, data: &[u8]) -> Option<Vec<u8>> {
    let bits = header.channels()? * header.depth;
    let stride = (header.width * bits).div_ceil(8);
    let raw = inflate::zlib(data, (stride + 1) * header.height)?;
    if raw.len() != (stride + 1) * header.height {
        return None;
    }
    let mut out = Vec::with_capacity(stride * header.height);
    let mut previous = vec![0; stride];
    for row in raw.chunks(stride + 1) {
        let mut line = row[1..].to_vec();
        unfilter(row[0], &mut line, &previous, bits.div_ceil(8))?;
        out.extend_from_slice(&line);
        previous = line;
    }
    Some(out)
}

fn palette_pixel(chunks: &Chunks, index: usize) -> Option<[u8; 4]> {
    let rgb = chunks.palette.get(index * 3..index * 3 + 3)?;
    let alpha = chunks.alpha.get(index).copied().unwrap_or(255);
    Some([rgb[0], rgb[1], rgb[2], alpha])
}

fn pixel(header: &Header, chunks: &Chunks, line: &[u8], x: usize) -> Option<[u8; 4]> {
    if header.color == 3 {
        let per_byte = 8 / header.depth;
        let shift = 8 - header.depth * (x % per_byte + 1);
        let mask = (1u16 << header.depth) - 1;
        let index = (u16::from(line[x / per_byte]) >> shift) & mask;
        return palette_pixel(chunks, usize::from(index));
    }
    let channels = header.channels()?;
    let s = &line[x * channels..(x + 1) * channels];
    Some(match channels {
        1 => [s[0], s[0], s[0], 255],
        2 => [s[0], s[0], s[0], s[1]],
        3 => [s[0], s[1], s[2], 255],
        _ => [s[0], s[1], s[2], s[3]],
    })
}

/// `None` — файл повреждён или использует неподдерживаемый вариант PNG.
pub fn decode_png(data: &[u8]) -> Option<Image> {
    let chunks = chunks(data)?;
    let header = Header::parse(&chunks.header)?;
    let raw = scanlines(&header, &chunks.data)?;
    let mut pixels = Vec::with_capacity(header.width * header.height * 4);
    for line in raw.chunks(raw.len() / header.height) {
        for x in 0..header.width {
            pixels.extend(pixel(&header, &chunks, line, x)?);
        }
    }
    Some(Image {
        width: header.width,
        height: header.height,
        pixels,
    })
}
