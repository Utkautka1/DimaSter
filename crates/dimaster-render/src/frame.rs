//! Превращает состояние терминала в список прямоугольников кадра.

use bytemuck::{Pod, Zeroable};
use dimaster_font::{FontSystem, Metrics};
use dimaster_grid::{Cell, CursorShape, Flags, Row, Span, Terminal};

use crate::appearance::Appearance;
use crate::atlas::{Atlas, Slot};
use crate::bidi;
use crate::palette::{self, Rgb};
use crate::text::Run;

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
    pub(crate) fn solid(position: [f32; 2], size: [f32; 2], color: Rgb) -> Self {
        Self {
            position,
            size,
            uv: [-1.0, 0.0],
            color,
        }
    }

    pub(crate) fn glyph(position: [f32; 2], slot: Slot, color: Rgb) -> Self {
        let size = [slot.width as f32, slot.height as f32];
        let color = color | u32::from(slot.color) << 24;
        Self {
            position,
            size,
            uv: [slot.x as f32, slot.y as f32],
            color,
        }
    }
}

/// Что именно показать из состояния терминала.
#[derive(Debug, Default, Clone, Copy)]
pub struct Viewport<'a> {
    /// На сколько строк пользователь отмотал историю назад.
    pub scroll: usize,
    pub selection: Option<Span>,
    /// Набираемый в IME текст, ещё не отправленный программе.
    pub preedit: &'a str,
}

/// Всё, что нужно для сборки кадра.
pub(crate) struct Builder<'a> {
    pub(crate) fonts: &'a mut FontSystem,
    pub(crate) atlas: &'a mut Atlas,
    pub(crate) quads: &'a mut Vec<Quad>,
    pub(crate) look: &'a Appearance,
}

impl Builder<'_> {
    pub(crate) fn build(&mut self, term: &Terminal, view: Viewport<'_>) {
        self.quads.clear();
        let scroll = view.scroll.min(term.screen().history().count());
        let first = term.visible_line(scroll);
        for index in 0..term.screen().rows().len() {
            let line = first + index as u64;
            let Some(row) = term.line(line) else { break };
            self.row(index, row, |col| {
                view.selection.is_some_and(|s| s.contains(line, col))
            });
        }
        if scroll == 0 && !view.preedit.is_empty() {
            self.preedit(term, view.preedit);
        } else if scroll == 0 && term.modes().cursor_visible {
            self.cursor(term);
        }
    }

    pub(crate) fn origin(&self, row: usize, col: usize) -> [f32; 2] {
        let m = self.fonts.metrics();
        [
            (self.look.padding.0 + col as u32 * m.cell_width) as f32,
            (self.look.padding.1 + row as u32 * m.cell_height) as f32,
        ]
    }

    fn row(&mut self, index: usize, row: &Row, selected: impl Fn(usize) -> bool) {
        let order = self.visual_order(&row.cells);
        let mut run = Run::default();
        for (col, cell) in row.cells.iter().enumerate() {
            if cell.style.flags.contains(Flags::WIDE_TAIL) {
                continue;
            }
            let column = order.as_ref().map_or(col, |o| o[col]);
            let origin = self.origin(index, column);
            let (mut fg, mut bg) = colors(cell, self.look);
            if selected(col) {
                bg = self.look.selection;
                fg = self.look.selection_text.unwrap_or(fg);
            }
            self.background(origin, cell, bg);
            if !cell.style.flags.contains(Flags::HIDDEN) {
                self.text_cell(&mut run, cell, fg, (index, column));
                self.lines(origin, cell, fg);
            }
        }
        self.flush(&mut run, index);
    }

    fn visual_order(&self, cells: &[Cell]) -> Option<Vec<usize>> {
        bidi::visual_columns(cells).filter(|_| self.look.rtl)
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
        if color != self.look.background {
            let size = [
                self.cell_width(cell),
                self.fonts.metrics().cell_height as f32,
            ];
            self.quads.push(Quad::solid(origin, size, color));
        }
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
        let order = self.visual_order(&term.screen().rows()[cursor.row].cells);
        let column = order.map_or(cursor.col, |o| o[cursor.col]);
        let origin = self.origin(cursor.row, column);
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
            .push(Quad::solid(position, size, self.look.cursor));
        // под блоком символ перерисовывается цветом фона
        if term.modes().cursor_shape == CursorShape::Block {
            let mut run = Run::default();
            self.text_cell(&mut run, cell, self.look.background, (cursor.row, column));
            self.flush(&mut run, cursor.row);
        }
    }
}

/// Итоговые цвета текста и фона с учётом INVERSE и DIM.
fn colors(cell: &Cell, look: &Appearance) -> (Rgb, Rgb) {
    let style = cell.style;
    let mut fg = palette::resolve(style.fg, look.foreground, &look.ansi);
    let mut bg = palette::resolve(style.bg, look.background, &look.ansi);
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
    use dimaster_grid::{Point, Selection, SelectionKind};

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
            look: &Appearance::default(),
        };
        builder.build(
            &term,
            Viewport {
                scroll,
                ..Viewport::default()
            },
        );
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
        assert_eq!(
            quads("a\r\n\r\n", 5)[0].color,
            Appearance::default().foreground
        );
    }

    // собирает кадр одной строки с заданными параметрами показа
    fn render(input: &str, view: impl FnOnce(&Terminal) -> Viewport<'static>) -> Vec<Quad> {
        let mut fonts = FontSystem::new(16.0).expect("в системе есть шрифт");
        let (mut atlas, mut quads) = (Atlas::default(), Vec::new());
        let mut term = Terminal::new(Size { rows: 1, cols: 8 });
        term.advance(input.as_bytes());
        let (view, look) = (view(&term), Appearance::default());
        let mut builder = Builder {
            fonts: &mut fonts,
            atlas: &mut atlas,
            quads: &mut quads,
            look: &look,
        };
        builder.build(&term, view);
        quads
    }

    #[test]
    fn selection_paints_background() {
        let quads = render("\x1b[?25lab", |term| {
            let point = |col| Point { line: 0, col };
            let kind = SelectionKind::Simple;
            let span = term.span(Selection {
                kind,
                anchor: point(0),
                head: point(1),
            });
            Viewport {
                selection: Some(span),
                ..Viewport::default()
            }
        });
        let painted = quads
            .iter()
            .filter(|q| q.color == Appearance::default().selection);
        assert_eq!(painted.count(), 2);
    }

    #[test]
    fn preedit_replaces_cursor() {
        let quads = render("", |_| Viewport {
            preedit: "длинный набор",
            ..Viewport::default()
        });
        let look = Appearance::default();
        assert_eq!(quads[0].color, look.selection);
        assert!(quads.iter().all(|q| q.color != look.cursor));
    }
}
