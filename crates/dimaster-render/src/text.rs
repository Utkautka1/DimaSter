//! Текст строки: клетки собираются в отрезки одного стиля и шейпятся вместе,
//! чтобы работали лигатуры и арабская вязь.

use dimaster_grid::{Cell, Flags};

use crate::frame::{Builder, Quad};
use crate::palette::Rgb;

/// Отрезок текста, который шейпится целиком.
#[derive(Default)]
pub(crate) struct Run {
    text: String,
    // начало символа в тексте -> видимая колонка и число ячеек
    cells: Vec<(usize, usize, u32)>,
    bold: bool,
    italic: bool,
    color: Rgb,
}

impl Run {
    fn accepts(&self, cell: &Cell, color: Rgb) -> bool {
        self.color == color
            && self.bold == cell.style.flags.contains(Flags::BOLD)
            && self.italic == cell.style.flags.contains(Flags::ITALIC)
    }

    fn push(&mut self, cell: &Cell, column: usize) {
        let span = if cell.style.flags.contains(Flags::WIDE) {
            2
        } else {
            1
        };
        let extra = cell.extra.iter().flat_map(|e| e.chars());
        for ch in std::iter::once(cell.ch).chain(extra) {
            self.cells.push((self.text.len(), column, span));
            self.text.push(ch);
        }
    }

    // ячейка, к которой относится глиф с данным кластером
    fn cell_at(&self, cluster: usize) -> Option<(usize, u32)> {
        let index = self
            .cells
            .partition_point(|c| c.0 <= cluster)
            .checked_sub(1)?;
        Some((self.cells[index].1, self.cells[index].2))
    }
}

impl Builder<'_> {
    /// Добавляет клетку в отрезок; пробел или смена стиля отрезок закрывает.
    pub(crate) fn text_cell(&mut self, run: &mut Run, cell: &Cell, color: Rgb, at: (usize, usize)) {
        let blank = cell.ch == ' ' && cell.extra.is_none();
        if blank || !run.accepts(cell, color) {
            self.flush(run, at.0);
        }
        if !blank {
            let flags = cell.style.flags;
            run.bold = flags.contains(Flags::BOLD);
            run.italic = flags.contains(Flags::ITALIC);
            run.color = color;
            run.push(cell, at.1);
        }
    }

    pub(crate) fn flush(&mut self, run: &mut Run, row: usize) {
        if run.text.is_empty() {
            return;
        }
        for glyph in self.fonts.shape(&run.text, run.bold, run.italic).iter() {
            let Some((column, cells)) = run.cell_at(glyph.cluster) else {
                continue;
            };
            let Some(slot) = self.atlas.get(self.fonts, glyph.id, cells) else {
                continue;
            };
            let origin = self.origin(row, column);
            let baseline = self.fonts.metrics().baseline;
            let x = origin[0] + glyph.x.round() + slot.left as f32;
            let y = origin[1] + (baseline - slot.top) as f32 - glyph.y.round();
            self.quads.push(Quad::glyph([x, y], slot, run.color));
        }
        run.text.clear();
        run.cells.clear();
    }
}
