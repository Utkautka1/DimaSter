//! Упаковка глифов в одну текстуру «полками».

use std::collections::HashMap;

use dimaster_font::{Bitmap, FontSystem, GlyphKey};

/// Сторона квадратной текстуры атласа в пикселях.
pub(crate) const ATLAS_SIZE: u32 = 2048;

/// Где лежит глиф и как его сдвинуть относительно ячейки.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Slot {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) left: i32,
    pub(crate) top: i32,
}

/// Новый растр, который ещё надо залить в текстуру.
pub(crate) struct Upload {
    pub(crate) slot: Slot,
    pub(crate) data: Vec<u8>,
}

/// Распределитель места в атласе.
#[derive(Default)]
pub(crate) struct Atlas {
    slots: HashMap<GlyphKey, Option<Slot>>,
    // позиция следующего глифа и высота текущей полки
    x: u32,
    y: u32,
    shelf: u32,
    pub(crate) uploads: Vec<Upload>,
    /// Место кончилось: кадр нужно собрать заново после очистки.
    pub(crate) full: bool,
}

impl Atlas {
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn get(&mut self, fonts: &mut FontSystem, key: GlyphKey) -> Option<Slot> {
        if let Some(slot) = self.slots.get(&key) {
            return *slot;
        }
        let bitmap = fonts.glyph(key);
        if bitmap.coverage.is_empty() {
            self.slots.insert(key, None);
            return None;
        }
        let slot = self.place(bitmap);
        match slot {
            Some(slot) => {
                self.uploads.push(Upload {
                    slot,
                    data: bitmap.coverage.clone(),
                });
                self.slots.insert(key, Some(slot));
            }
            None => self.full = true,
        }
        slot
    }

    fn place(&mut self, bitmap: &Bitmap) -> Option<Slot> {
        let (width, height) = (bitmap.width as u32, bitmap.height as u32);
        if self.x + width > ATLAS_SIZE {
            self.x = 0;
            self.y += self.shelf;
            self.shelf = 0;
        }
        if width > ATLAS_SIZE || self.y + height > ATLAS_SIZE {
            return None;
        }
        let slot = Slot {
            x: self.x,
            y: self.y,
            width,
            height,
            left: bitmap.left,
            top: bitmap.top,
        };
        self.x += width;
        self.shelf = self.shelf.max(height);
        Some(slot)
    }
}
