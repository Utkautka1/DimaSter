//! Всё, что рисуется поверх терминала: панели, полоса заголовка, курсор режима.

use dimaster_font::Metrics;
use dimaster_grid::{Flags, Size, Terminal};

use crate::frame::{Builder, Quad};
use crate::palette::Rgb;
use crate::text::Run;

/// Строка интерфейса в пикселях окна.
#[derive(Debug, Clone)]
pub struct Label {
    pub x: u32,
    pub y: u32,
    // ширина подложки в ячейках; 0 — подложки нет
    pub width: usize,
    pub text: String,
    pub color: Rgb,
    pub background: Rgb,
}

impl Builder<'_> {
    pub(crate) fn label(&mut self, label: &Label) {
        let Metrics {
            cell_width,
            cell_height,
            ..
        } = self.fonts.metrics();
        let at = [label.x as f32, label.y as f32];
        if label.width > 0 {
            let size = [(label.width as u32 * cell_width) as f32, cell_height as f32];
            self.quads.push(Quad::solid(at, size, label.background));
        }
        // ширины символов считает тот же эмулятор, что и в сетке
        let cols = label.text.chars().count() * 2 + 2;
        let mut grid = Terminal::new(Size { rows: 1, cols });
        grid.advance(label.text.replace(char::is_control, " ").as_bytes());
        let limit = if label.width > 0 { label.width } else { cols };
        let mut run = Run::default();
        let first = self.quads.len();
        for (col, cell) in grid.screen().rows()[0].cells.iter().take(limit).enumerate() {
            if !cell.style.flags.contains(Flags::WIDE_TAIL) {
                self.text_cell(&mut run, cell, label.color, (0, col));
            }
        }
        self.flush(&mut run, 0);
        self.shift(first, at);
    }

    // глифы собраны относительно сетки — переносим их в нужное место окна
    fn shift(&mut self, first: usize, to: [f32; 2]) {
        let origin = self.origin(0, 0);
        for quad in &mut self.quads[first..] {
            quad.position[0] += to[0] - origin[0];
            quad.position[1] += to[1] - origin[1];
        }
    }
}
