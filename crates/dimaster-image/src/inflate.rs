//! Распаковка DEFLATE (RFC 1951) для zlib-потока внутри PNG.

// порядок длин кодов в заголовке динамического блока
const ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];

/// Читатель битов, младший бит первым.
struct Bits<'a> {
    data: &'a [u8],
    position: usize,
}

impl Bits<'_> {
    fn bit(&mut self) -> Option<u32> {
        let byte = *self.data.get(self.position / 8)?;
        let bit = (byte >> (self.position % 8)) & 1;
        self.position += 1;
        Some(u32::from(bit))
    }

    fn take(&mut self, count: u32) -> Option<u32> {
        (0..count).try_fold(0, |value, shift| Some(value | self.bit()? << shift))
    }

    // несжатый блок начинается с границы байта
    fn stored(&mut self) -> Option<&[u8]> {
        self.position = self.position.next_multiple_of(8);
        let length = self.take(16)? as usize;
        self.take(16)?;
        let start = self.position / 8;
        self.position += length * 8;
        self.data.get(start..start + length)
    }
}

/// Канонический код Хаффмана: число кодов каждой длины и символы по порядку.
struct Huffman {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Self {
        let mut counts = [0; 16];
        for &length in lengths.iter().filter(|&&l| l > 0) {
            counts[usize::from(length)] += 1;
        }
        let mut order: Vec<u16> = (0..lengths.len() as u16)
            .filter(|&s| lengths[usize::from(s)] > 0)
            .collect();
        order.sort_by_key(|&s| lengths[usize::from(s)]);
        Self {
            counts,
            symbols: order,
        }
    }

    fn decode(&self, bits: &mut Bits<'_>) -> Option<u16> {
        let (mut code, mut first, mut index) = (0u32, 0u32, 0u32);
        for &count in &self.counts[1..] {
            code |= bits.bit()?;
            let count = u32::from(count);
            if code.wrapping_sub(first) < count {
                return self.symbols.get((index + code - first) as usize).copied();
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        None
    }
}

fn fixed_tables() -> (Huffman, Huffman) {
    let mut lengths = [8u8; 288];
    lengths[144..256].fill(9);
    lengths[256..280].fill(7);
    (Huffman::new(&lengths), Huffman::new(&[5; 30]))
}

fn dynamic_tables(bits: &mut Bits<'_>) -> Option<(Huffman, Huffman)> {
    let literals = bits.take(5)? as usize + 257;
    let distances = bits.take(5)? as usize + 1;
    let mut code_lengths = [0u8; 19];
    for &slot in ORDER.iter().take(bits.take(4)? as usize + 4) {
        code_lengths[slot] = bits.take(3)? as u8;
    }
    let lengths = read_lengths(bits, &Huffman::new(&code_lengths), literals + distances)?;
    let (first, second) = lengths.split_at(literals);
    Some((Huffman::new(first), Huffman::new(second)))
}

// длины кодов сами сжаты повторами (символы 16, 17, 18)
fn read_lengths(bits: &mut Bits<'_>, table: &Huffman, total: usize) -> Option<Vec<u8>> {
    let mut lengths = Vec::with_capacity(total);
    while lengths.len() < total {
        let (value, repeat) = match table.decode(bits)? {
            symbol @ 0..=15 => (symbol as u8, 1),
            16 => (*lengths.last()?, 3 + bits.take(2)?),
            17 => (0, 3 + bits.take(3)?),
            _ => (0, 11 + bits.take(7)?),
        };
        lengths.extend(std::iter::repeat_n(value, repeat as usize));
    }
    (lengths.len() == total).then_some(lengths)
}

fn copy_match(
    bits: &mut Bits<'_>,
    symbol: u16,
    distances: &Huffman,
    out: &mut Vec<u8>,
) -> Option<()> {
    let index = usize::from(symbol - 257);
    let extra = if index < 8 || index == 28 {
        0
    } else {
        index as u32 / 4 - 1
    };
    let length = usize::from(*LENGTH_BASE.get(index)?) + bits.take(extra)? as usize;
    let code = usize::from(distances.decode(bits)?);
    let extra = (code as u32 / 2).saturating_sub(1);
    let distance = usize::from(*DISTANCE_BASE.get(code)?) + bits.take(extra)? as usize;
    let start = out.len().checked_sub(distance)?;
    // источник может перекрывать результат, поэтому копируем по байту
    for offset in 0..length {
        out.push(out[start + offset]);
    }
    Some(())
}

fn block(
    bits: &mut Bits<'_>,
    tables: (Huffman, Huffman),
    out: &mut Vec<u8>,
    limit: usize,
) -> Option<()> {
    loop {
        match tables.0.decode(bits)? {
            256 => return Some(()),
            symbol @ 0..=255 => out.push(symbol as u8),
            symbol => copy_match(bits, symbol, &tables.1, out)?,
        }
        if out.len() > limit {
            return None;
        }
    }
}

/// Распаковывает zlib-поток; `limit` защищает от «zip-бомбы».
pub(crate) fn zlib(data: &[u8], limit: usize) -> Option<Vec<u8>> {
    let mut bits = Bits {
        data: data.get(2..)?,
        position: 0,
    };
    let mut out = Vec::new();
    loop {
        let last = bits.bit()? == 1;
        match bits.take(2)? {
            0 => out.extend_from_slice(bits.stored()?),
            1 => block(&mut bits, fixed_tables(), &mut out, limit)?,
            2 => {
                let tables = dynamic_tables(&mut bits)?;
                block(&mut bits, tables, &mut out, limit)?;
            }
            _ => return None,
        }
        if out.len() > limit {
            return None;
        }
        if last {
            return Some(out);
        }
    }
}
