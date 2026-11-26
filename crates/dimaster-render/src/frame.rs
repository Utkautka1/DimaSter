//! Превращает состояние терминала в список прямоугольников кадра.

use bytemuck::{Pod, Zeroable};
use dimaster_font::{FontSystem, Metrics};
use dimaster_grid::{Cell, CursorShape, Flags, Row, Span, Terminal};

use crate::appearance::Appearance;
use crate::atlas::{Atlas, Slot};
use crate::bidi;
use crate::images::Images;
use crate::overlay::Label;
use crate::palette::{self, Rgb};
use crate::text::Run;

/// Бит цвета: заливка рисуется полупрозрачной.
pub(crate) const TRANSLUCENT: u32 = 1 << 26;

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
    /// Подсветка совпадений поиска и ссылок.
    pub highlights: &'a [(Span, Rgb)],
    pub labels: &'a [Label],
    // сдвиг содержимого в пикселях во время плавной прокрутки
    pub offset: f32,
    /// Где рисовать курсор: (строка, колонка) в ячейках, дробные при анимации.
    /// `None` — там, где он в терминале.
    pub cursor: Option<(f32, f32)>,
    pub cursor_hidden: bool,
    /// Прошлые положения курсора для «хвоста».
    pub trail: &'a [(f32, f32)],
}

/// Всё, что нужно для сборки кадра.
pub(crate) struct Builder<'a> {
    pub(crate) fonts: &'a mut FontSystem,
    pub(crate) atlas: &'a mut Atlas,
    pub(crate) quads: &'a mut Vec<Quad>,
    pub(crate) look: &'a Appearance,
    pub(crate) pictures: &'a mut Images,
}

impl Builder<'_> {
    pub(crate) fn build(&mut self, term: &Terminal, view: Viewport<'_>) {
        self.quads.clear();
        self.pictures.clear();
        let scroll = view.scroll.min(term.screen().history().count());
        let first = term.visible_line(scroll);
        self.rows(term, view, first);
        self.images(term, first);
        let live = scroll == 0 || view.cursor.is_some();
        if live && !view.preedit.is_empty() {
            self.preedit(term, view.preedit);
        } else if live && !view.cursor_hidden {
            self.trail(view.trail);
            let program_hides = !term.modes().cursor_visible && view.cursor.is_none();
            if !program_hides {
                self.cursor(term, view.cursor);
            }
        }
    }

    /// Панели и заголовок идут отдельным слоем поверх картинок.
    pub(crate) fn overlay(&mut self, labels: &[Label]) {
        self.quads.clear();
        labels.iter().for_each(|label| self.label(label));
    }

    fn rows(&mut self, term: &Terminal, view: Viewport<'_>, first: u64) {
        let count = term.screen().rows().len() as i64;
        // при сдвиге видны и соседние строки
        let extra = if view.offset == 0.0 { 0 } else { 3 };
        for index in -extra..count + extra {
            let Some(line) = first.checked_add_signed(index) else {
                continue;
            };
            let Some(row) = term.line(line) else { continue };
            let start = self.quads.len();
            let selection = self.look.selection;
            self.row(index.max(0) as usize, row, |col| {
                let highlight = view.highlights.iter().find(|h| h.0.contains(line, col));
                let selected = view.selection.is_some_and(|s| s.contains(line, col));
                highlight.map(|h| h.1).or(selected.then_some(selection))
            });
            self.move_down(start, index.min(0) as f32, view.offset);
        }
    }

    fn move_down(&mut self, first: usize, rows: f32, pixels: f32) {
        let shift = rows * self.fonts.metrics().cell_height as f32 + pixels;
        if shift != 0.0 {
            self.quads[first..]
                .iter_mut()
                .for_each(|q| q.position[1] += shift);
        }
    }

    fn trail(&mut self, trail: &[(f32, f32)]) {
        let Metrics {
            cell_width,
            cell_height,
            ..
        } = self.fonts.metrics();
        for &(row, col) in trail {
            let origin = self.origin(0, 0);
            let at = [
                origin[0] + col * cell_width as f32,
                origin[1] + row * cell_height as f32,
            ];
            let size = [cell_width as f32, cell_height as f32];
            let color = self.look.cursor | TRANSLUCENT;
            self.quads.push(Quad::solid(at, size, color));
        }
    }

    pub(crate) fn origin(&self, row: usize, col: usize) -> [f32; 2] {
        let m = self.fonts.metrics();
        [
            (self.look.padding.0 + col as u32 * m.cell_width) as f32,
            (self.look.padding.1 + self.look.top + row as u32 * m.cell_height) as f32,
        ]
    }

    fn row(&mut self, index: usize, row: &Row, marked: impl Fn(usize) -> Option<Rgb>) {
        let order = self.visual_order(&row.cells);
        let mut run = Run::default();
        for (col, cell) in row.cells.iter().enumerate() {
            if cell.style.flags.contains(Flags::WIDE_TAIL) {
                continue;
            }
            let column = order.as_ref().map_or(col, |o| o[col]);
            let origin = self.origin(index, column);
            let (mut fg, mut bg) = colors(cell, self.look);
            if let Some(color) = marked(col) {
                bg = color;
                // свой цвет текста есть только у выделения, не у подсветки поиска
                if color == self.look.selection {
                    fg = self.look.selection_text.unwrap_or(fg);
                }
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

    fn cursor(&mut self, term: &Terminal, moved: Option<(f32, f32)>) {
        let cursor = term.cursor();
        let cell = &term.screen().rows()[cursor.row].cells[cursor.col];
        let order = self.visual_order(&term.screen().rows()[cursor.row].cells);
        let column = order.map_or(cursor.col, |o| o[cursor.col]);
        let resting = (cursor.row as f32, column as f32);
        let (row, col) = moved.unwrap_or(resting);
        let metrics = self.fonts.metrics();
        let zero = self.origin(0, 0);
        let origin = [
            zero[0] + col * metrics.cell_width as f32,
            zero[1] + row * metrics.cell_height as f32,
        ];
        let size = [self.cell_width(cell), metrics.cell_height as f32];
        self.cursor_shape(term.modes().cursor_shape, origin, size);
        // под неподвижным блоком символ перерисовывается цветом фона
        if term.modes().cursor_shape == CursorShape::Block && (row, col) == resting {
            let mut run = Run::default();
            self.text_cell(&mut run, cell, self.look.background, (cursor.row, column));
            self.flush(&mut run, cursor.row);
        }
    }

    fn cursor_shape(&mut self, shape: CursorShape, origin: [f32; 2], [width, height]: [f32; 2]) {
        let thin = (self.fonts.metrics().line_thickness * 2) as f32;
        let (position, size) = match shape {
            CursorShape::Block => (origin, [width, height]),
            CursorShape::Underline => ([origin[0], origin[1] + height - thin], [width, thin]),
            CursorShape::Bar => (origin, [thin, height]),
        };
        self.quads
            .push(Quad::solid(position, size, self.look.cursor));
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
            pictures: &mut Images::default(),
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
            pictures: &mut Images::default(),
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
