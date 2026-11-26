//! Шейпинг отрезка текста: выбор шрифта для символов и расстановка глифов.

use std::rc::Rc;

use fontdb::ID;
use rustybuzz::{Face, GlyphBuffer, UnicodeBuffer};

use crate::{FontSystem, discover};

/// Конкретный глиф конкретного шрифта.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphId {
    pub(crate) face: ID,
    pub(crate) glyph: u16,
}

/// Глиф после шейпинга.
#[derive(Debug, Clone, Copy)]
pub struct ShapedGlyph {
    pub id: GlyphId,
    /// Байтовое смещение символа в исходном тексте, которому принадлежит глиф.
    pub cluster: usize,
    /// Смещение от начала ячейки кластера в пикселях.
    pub x: f32,
    /// Смещение вверх от базовой линии в пикселях.
    pub y: f32,
}

// кэш сбрасывается целиком: строки на экране повторяются, мусор — нет
const CACHE_LIMIT: usize = 8192;

// знаки, которые должны шейпиться тем же шрифтом, что и предыдущий символ
fn joins_previous(ch: char) -> bool {
    matches!(ch, '\u{200c}' | '\u{200d}' | '\u{fe00}'..='\u{fe0f}' | '\u{0300}'..='\u{036f}')
        || matches!(ch, '\u{1f3fb}'..='\u{1f3ff}' | '\u{064b}'..='\u{065f}' | '\u{0591}'..='\u{05c7}')
}

impl FontSystem {
    /// Превращает текст одного стиля в глифы; результат кэшируется.
    pub fn shape(&mut self, text: &str, bold: bool, italic: bool) -> Rc<[ShapedGlyph]> {
        let key = (text.to_owned(), bold, italic);
        if let Some(glyphs) = self.shaped.get(&key) {
            return glyphs.clone();
        }
        let mut glyphs = Vec::new();
        for (face, start, end) in self.segments(text, bold, italic) {
            self.shape_segment(face, &text[start..end], start, &mut glyphs);
        }
        if self.shaped.len() >= CACHE_LIMIT {
            self.shaped.clear();
        }
        let glyphs: Rc<[ShapedGlyph]> = glyphs.into();
        self.shaped.insert(key, glyphs.clone());
        glyphs
    }

    // делит текст на куски, каждый из которых рисуется одним шрифтом
    fn segments(&mut self, text: &str, bold: bool, italic: bool) -> Vec<(ID, usize, usize)> {
        let Some(primary) = discover::find_face(&self.db, &self.family, bold, italic) else {
            return Vec::new();
        };
        let mut segments: Vec<(ID, usize, usize)> = Vec::new();
        for (at, ch) in text.char_indices() {
            let end = at + ch.len_utf8();
            // U+FE0F после символа просит его цветное начертание
            let emoji = discover::is_emoji(ch) || text[end..].starts_with('\u{fe0f}');
            let face = match segments.last() {
                Some(&(face, ..)) if joins_previous(ch) => face,
                _ => self.face_for(primary, ch, emoji),
            };
            match segments.last_mut() {
                Some(last) if last.0 == face => last.2 = end,
                _ => segments.push((face, at, end)),
            }
        }
        segments
    }

    fn face_for(&mut self, primary: ID, ch: char, emoji: bool) -> ID {
        if !emoji && (ch.is_ascii() || discover::has_char(&self.db, primary, ch)) {
            return primary;
        }
        let db = &self.db;
        let found = self.fallbacks.entry((ch, emoji)).or_insert_with(|| {
            let color = emoji.then(|| discover::emoji_face(db, ch)).flatten();
            color.or_else(|| discover::fallback_face(db, ch))
        });
        found.unwrap_or(primary)
    }

    fn shape_segment(&self, id: ID, text: &str, offset: usize, out: &mut Vec<ShapedGlyph>) {
        self.db.with_face_data(id, |data, index| {
            let Some(face) = Face::from_slice(data, index) else {
                return;
            };
            let mut buffer = UnicodeBuffer::new();
            buffer.push_str(text);
            buffer.guess_segment_properties();
            let shaped = rustybuzz::shape(&face, &[], buffer);
            let scale = self.size / face.units_per_em() as f32;
            collect(id, &shaped, scale, offset, out);
        });
    }
}

// переводит позиции из единиц шрифта в пиксели относительно ячейки кластера
fn collect(id: ID, shaped: &GlyphBuffer, scale: f32, offset: usize, out: &mut Vec<ShapedGlyph>) {
    // внутри одного кластера глифы идут друг за другом
    let (mut cluster, mut pen) = (u32::MAX, 0);
    for (info, place) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
        if info.cluster != cluster {
            (cluster, pen) = (info.cluster, 0);
        }
        out.push(ShapedGlyph {
            id: GlyphId {
                face: id,
                glyph: info.glyph_id as u16,
            },
            cluster: offset + info.cluster as usize,
            x: (pen + place.x_offset) as f32 * scale,
            y: place.y_offset as f32 * scale,
        });
        pen += place.x_advance;
    }
}
