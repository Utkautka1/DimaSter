//! Набор IME: ещё не отправленный текст поверх строки курсора.

use dimaster_font::Metrics;
use dimaster_grid::{Flags, Row, Size, Terminal};

use crate::frame::{Builder, Quad};
use crate::text::Run;

// раскладывает набранное по клеткам теми же правилами ширины, что и терминал
fn layout(text: &str, cols: usize) -> (Row, usize) {
    let mut grid = Terminal::new(Size { rows: 1, cols });
    grid.advance(text.replace(char::is_control, "").as_bytes());
    let width = grid.cursor().col.max(1);
    (grid.screen().rows()[0].clone(), width)
}

impl Builder<'_> {
    pub(crate) fn preedit(&mut self, term: &Terminal, text: &str) {
        let cursor = term.cursor();
        let cols = term.screen().rows()[0].cells.len();
        let (row, width) = layout(text, cols);
        // у правого края сдвигаем влево, чтобы набор был виден целиком
        let start = cursor.col.min(cols.saturating_sub(width));
        self.preedit_box(cursor.row, start, width);
        let mut run = Run::default();
        for (col, cell) in row.cells.iter().take(width).enumerate() {
            if !cell.style.flags.contains(Flags::WIDE_TAIL) {
                self.text_cell(
                    &mut run,
                    cell,
                    self.look.foreground,
                    (cursor.row, start + col),
                );
            }
        }
        self.flush(&mut run, cursor.row);
    }

    // фон и подчёркивание под набираемым текстом
    fn preedit_box(&mut self, row: usize, col: usize, width: usize) {
        let Metrics {
            cell_width,
            cell_height,
            line_thickness,
            ..
        } = self.fonts.metrics();
        let origin = self.origin(row, col);
        let size = [(width as u32 * cell_width) as f32, cell_height as f32];
        self.quads
            .push(Quad::solid(origin, size, self.look.selection));
        let line = [origin[0], origin[1] + size[1] - line_thickness as f32];
        let thin = [size[0], line_thickness as f32];
        self.quads
            .push(Quad::solid(line, thin, self.look.foreground));
    }
}
