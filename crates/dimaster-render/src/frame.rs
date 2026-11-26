//! Превращает состояние терминала в список прямоугольников кадра.

use bytemuck::{Pod, Zeroable};
use dimaster_font::{FontSystem, GlyphKey, Metrics};
use dimaster_grid::{Cell, CursorShape, Flags, Row, Terminal};

use crate::atlas::Atlas;
use crate::palette::{self, Rgb};

/// Отступ от краёв окна в пикселях.
pub const PADDING: u32 = 6;

/// Прямоугольник кадра в том виде, в каком его читает шейдер.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct Quad {
    pub(crate) position: [f32; 2],
    pub(crate) size: [f32; 2],
    /// Левый верхний угол в атласе; отрицательный X — сплошная заливка.
    pub(crate) uv: [f32; 2],
    pub(crate) color: u32,
}

impl Quad {
    fn solid(position: [f32; 2], size: [f32; 2], color: Rgb) -> Self {
        Self {
            position,
            size,
            uv: [-1.0, 0.0],
            color,
        }
    }
}

/// Всё, что нужно для сборки кадра.
pub(crate) struct Builder<'a> {
    pub(crate) fonts: &'a mut FontSystem,
    pub(crate) atlas: &'a mut Atlas,
    pub(crate) quads: &'a mut Vec<Quad>,
}

impl Builder<'_> {
    /// `scroll` — на сколько строк пользователь отмотал историю назад.
    pub(crate) fn build(&mut self, term: &Terminal, scroll: usize) {
        self.quads.clear();
        let screen = term.screen();
        let scroll = scroll.min(screen.history().count());
        let history = screen.history().rev().take(scroll).collect::<Vec<_>>();
        let visible = history
            .into_iter()
            .rev()
            .chain(screen.rows())
            .take(screen.rows().len());
        for (index, row) in visible.enumerate() {
            self.row(index, row);
        }
        if scroll == 0 && term.modes().cursor_visible {
            self.cursor(term);
        }
    }

    fn origin(&self, row: usize, col: usize) -> [f32; 2] {
        let m = self.fonts.metrics();
        [
            (PADDING + col as u32 * m.cell_width) as f32,
            (PADDING + row as u32 * m.cell_height) as f32,
        ]
    }

    fn row(&mut self, index: usize, row: &Row) {
        for (col, cell) in row.cells.iter().enumerate() {
            if cell.style.flags.contains(Flags::WIDE_TAIL) {
                continue;
            }
            let origin = self.origin(index, col);
            let (fg, bg) = colors(cell);
            self.background(origin, cell, bg);
            if !cell.style.flags.contains(Flags::HIDDEN) {
                self.glyphs(origin, cell, fg);
                self.lines(origin, cell, fg);
            }
        }
    }

    fn cell_width(&self, cell: &Cell) -> f32 {
        let span = if cell.style.flags.contains(Flags::WIDE) {
            2
        } else {
            1
        };
        (self.fonts.metrics().cell_width * span) as f32
    }

    fn background(&mut self, origin: [f32; 2], cell: &Cell, color: Rgb) {
        // фон по умолчанию уже залит очисткой кадра
        if color != palette::BACKGROUND {
            let size = [
                self.cell_width(cell),
                self.fonts.metrics().cell_height as f32,
            ];
            self.quads.push(Quad::solid(origin, size, color));
        }
    }

    fn glyphs(&mut self, origin: [f32; 2], cell: &Cell, color: Rgb) {
        let extra = cell.extra.iter().flat_map(|e| e.chars());
        for ch in std::iter::once(cell.ch).chain(extra).filter(|&c| c != ' ') {
            let key = GlyphKey {
                ch,
                bold: cell.style.flags.contains(Flags::BOLD),
                italic: cell.style.flags.contains(Flags::ITALIC),
            };
            self.glyph(origin, key, color);
        }
    }

    fn glyph(&mut self, origin: [f32; 2], key: GlyphKey, color: Rgb) {
        let Some(slot) = self.atlas.get(self.fonts, key) else {
            return;
        };
        let baseline = self.fonts.metrics().baseline;
        self.quads.push(Quad {
            position: [
                origin[0] + slot.left as f32,
                origin[1] + (baseline - slot.top) as f32,
            ],
            size: [slot.width as f32, slot.height as f32],
            uv: [slot.x as f32, slot.y as f32],
            color,
        });
    }

    fn lines(&mut self, origin: [f32; 2], cell: &Cell, color: Rgb) {
        let Metrics {
            baseline,
            line_thickness,
            ..
        } = self.fonts.metrics();
        let size = [self.cell_width(cell), line_thickness as f32];
        let underline = (Flags::UNDERLINE, baseline + line_thickness as i32);
        let strike = (Flags::STRIKE, baseline * 2 / 3);
        for (flag, offset) in [underline, strike] {
            if cell.style.flags.contains(flag) {
                let position = [origin[0], origin[1] + offset as f32];
                self.quads.push(Quad::solid(position, size, color));
            }
        }
    }

    fn cursor(&mut self, term: &Terminal) {
        let cursor = term.cursor();
        let cell = &term.screen().rows()[cursor.row].cells[cursor.col];
        let origin = self.origin(cursor.row, cursor.col);
        let Metrics {
            cell_height,
            line_thickness,
            ..
        } = self.fonts.metrics();
        let (width, height) = (self.cell_width(cell), cell_height as f32);
        let thin = (line_thickness * 2) as f32;
        let (position, size) = match term.modes().cursor_shape {
            CursorShape::Block => (origin, [width, height]),
            CursorShape::Underline => ([origin[0], origin[1] + height - thin], [width, thin]),
            CursorShape::Bar => (origin, [thin, height]),
        };
        self.quads
            .push(Quad::solid(position, size, palette::CURSOR));
        // под блоком символ перерисовывается цветом фона
        if term.modes().cursor_shape == CursorShape::Block {
            self.glyphs(origin, cell, palette::BACKGROUND);
        }
    }
}

/// Итоговые цвета текста и фона с учётом INVERSE и DIM.
fn colors(cell: &Cell) -> (Rgb, Rgb) {
    let style = cell.style;
    let mut fg = palette::resolve(style.fg, palette::FOREGROUND);
    let mut bg = palette::resolve(style.bg, palette::BACKGROUND);
    if style.flags.contains(Flags::INVERSE) {
        std::mem::swap(&mut fg, &mut bg);
    }
    if style.flags.contains(Flags::DIM) {
        fg = palette::dim(fg, bg);
    }
    (fg, bg)
}

#[cfg(test)]
mod tests {
    use dimaster_grid::Size;

    use super::*;

    fn quads(input: &str, scroll: usize) -> Vec<Quad> {
        let mut fonts = FontSystem::new(16.0).expect("в системе есть шрифт");
        let (mut atlas, mut quads) = (Atlas::default(), Vec::new());
        let mut term = Terminal::new(Size { rows: 2, cols: 8 });
        term.advance(input.as_bytes());
        let mut builder = Builder {
            fonts: &mut fonts,
            atlas: &mut atlas,
            quads: &mut quads,
        };
        builder.build(&term, scroll);
        quads
    }

    #[test]
    fn empty_screen_draws_only_cursor() {
        assert_eq!(quads("", 0).len(), 1);
        assert_eq!(quads("\x1b[?25l", 0).len(), 0);
    }

    #[test]
    fn background_glyph_and_underline() {
        assert_eq!(quads("\x1b[?25l\x1b[41;4mA", 0).len(), 3);
    }

    #[test]
    fn scrolled_view_shows_history_without_cursor() {
        assert_eq!(quads("a\r\n\r\n", 0).len(), 1);
        assert_eq!(quads("a\r\n\r\n", 5).len(), 1);
        assert_eq!(quads("a\r\n\r\n", 5)[0].color, palette::FOREGROUND);
    }
}
