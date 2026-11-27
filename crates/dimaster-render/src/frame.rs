//! Превращает состояние терминала в список прямоугольников кадра.

use bytemuck::{Pod, Zeroable};
use dimaster_font::FontSystem;
use dimaster_grid::{Cell, Color, Flags, Row, Span, Style, Terminal};

use crate::appearance::Appearance;
use crate::atlas::{Atlas, Slot};
use crate::bidi;
use crate::images::Images;
use crate::overlay::{Label, Rect};
use crate::palette::{self, Rgb};
use crate::text::Run;

/// Бит цвета: заливка рисуется полупрозрачной.
pub(crate) const TRANSLUCENT: u32 = 1 << 26;
// отрезок вместо заливки и его наклон; те же биты читает шейдер
pub(crate) const LINE: u32 = 1 << 27;
pub(crate) const RISING: u32 = 1 << 28;

/// Прямоугольник кадра в том виде, в каком его читает шейдер.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct Quad {
    pub(crate) position: [f32; 2],
    pub(crate) size: [f32; 2],
    /// Левый верхний угол в атласе; отрицательный X — сплошная заливка.
    pub(crate) uv: [f32; 2],
    pub(crate) color: u32,
    pub(crate) uv_size: [f32; 2],
}

impl Quad {
    pub(crate) fn solid(position: [f32; 2], size: [f32; 2], color: Rgb) -> Self {
        Self {
            position,
            size,
            uv: [-1.0, 0.0],
            color,
            uv_size: [0.0, 0.0],
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
            uv_size: [0.0, 0.0],
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
    /// Призрачная автоподсказка справа от курсора.
    pub ghost: Option<&'a str>,
    /// Подсветка совпадений поиска и ссылок.
    pub highlights: &'a [(Span, Rgb)],
    /// Подсветка синтаксиса вводимой команды (цвет текста для ячеек с `Color::Default`).
    pub syntax: &'a [(Span, Rgb)],
    pub labels: &'a [Label],
    // сдвиг содержимого в пикселях во время плавной прокрутки
    pub offset: f32,
    /// Где рисовать курсор: (строка, колонка) в ячейках, дробные при анимации.
    /// `None` — там, где он в терминале.
    pub cursor: Option<(f32, f32)>,
    pub cursor_hidden: bool,
    /// Окно не в фокусе.
    pub unfocused: bool,
    /// Доля высоты, на которую курсор сжат к середине; 0 — целый.
    pub shrink: f32,
    /// Прошлые положения курсора для «хвоста».
    pub trail: &'a [(f32, f32)],
    /// Полоса прокрутки, свечение рамки.
    pub rects: &'a [Rect],
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
        let scroll = view.scroll.min(term.screen().history_len());
        let first = term.visible_line(scroll);
        self.rows(term, view, first);
        self.images(term, first);
        let live = scroll == 0 || view.cursor.is_some();
        if live && !view.preedit.is_empty() {
            self.preedit(term, view.preedit);
        } else if live {
            if let Some(ghost) = view.ghost {
                self.ghost(term, ghost);
            }
            if !view.cursor_hidden {
                self.trail(term, view.trail);
                let program_hides = !term.modes().cursor_visible && view.cursor.is_none();
                if !program_hides {
                    self.cursor(term, view.cursor, (view.unfocused, view.shrink));
                }
            }
        }
    }

    /// Панели и заголовок идут отдельным слоем поверх картинок.
    pub(crate) fn overlay(&mut self, rects: &[Rect], labels: &[Label]) {
        self.quads.clear();
        rects.iter().for_each(|rect| self.rect(rect));
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
            self.row(
                index.max(0) as usize,
                &row,
                |col| mark_at(view, line, col, selection),
                |col| syntax_at(view.syntax, line, col),
            );
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

    pub(crate) fn origin(&self, row: usize, col: usize) -> [f32; 2] {
        let m = self.fonts.metrics();
        [
            (self.look.padding.0 + col as u32 * m.cell_width) as f32,
            (self.look.padding.1 + self.look.top + row as u32 * m.cell_height) as f32,
        ]
    }

    fn cell_colors(&self, cell: &Cell, marked: Option<Rgb>, syntax: Option<Rgb>) -> (Rgb, Rgb) {
        let (mut fg, mut bg) = colors(cell, self.look);
        if let Some(color) = syntax
            && cell.style.fg == Color::Default
            && !cell.style.flags.contains(Flags::INVERSE)
        {
            fg = color;
        }
        if let Some(color) = marked {
            bg = color;
            // свой цвет текста есть только у выделения, не у подсветки поиска
            if color == self.look.selection {
                fg = self.look.selection_text.unwrap_or(fg);
            }
        }
        (fg, bg)
    }

    fn row(
        &mut self,
        index: usize,
        row: &Row,
        marked: impl Fn(usize) -> Option<Rgb>,
        syntax: impl Fn(usize) -> Option<Rgb>,
    ) {
        let order = self.visual_order(&row.cells);
        let mut run = Run::default();
        for (col, cell) in row.cells.iter().enumerate() {
            if cell.style.flags.contains(Flags::WIDE_TAIL) {
                continue;
            }
            let column = order.as_ref().map_or(col, |o| o[col]);
            let origin = self.origin(index, column);
            let (fg, bg) = self.cell_colors(cell, marked(col), syntax(col));
            self.background(origin, cell, bg);
            if !cell.style.flags.contains(Flags::HIDDEN) {
                // значку Nerd Font перед пробелом можно занять и его
                let icon = matches!(cell.ch, '\u{e000}'..='\u{f8ff}' | '\u{f0000}'..='\u{ffffd}');
                let roomy = icon && row.cells.get(col + 1).is_none_or(|next| next.ch == ' ');
                self.text_cell(&mut run, cell, fg, (index, column), roomy);
                self.lines(origin, cell, fg);
            }
        }
        self.flush(&mut run, index);
    }

    pub(crate) fn visual_order(&self, cells: &[Cell]) -> Option<Vec<usize>> {
        bidi::visual_columns(cells).filter(|_| self.look.rtl)
    }

    pub(crate) fn cell_width(&self, cell: &Cell) -> f32 {
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

    pub(crate) fn underline_color(&self, style: Style, text: Rgb) -> Rgb {
        match style.underline_color {
            Color::Default => text,
            color => resolve(color, text, self.look),
        }
    }
}

fn resolve(color: Color, default: Rgb, look: &Appearance) -> Rgb {
    let changed = match color {
        Color::Indexed(index) => look.palette.get(&index).copied(),
        _ => None,
    };
    changed.unwrap_or_else(|| palette::resolve(color, default, &look.ansi))
}

fn mark_at(view: Viewport<'_>, line: u64, col: usize, selection: Rgb) -> Option<Rgb> {
    let highlight = view.highlights.iter().find(|h| h.0.contains(line, col));
    let selected = view.selection.is_some_and(|s| s.contains(line, col));
    highlight.map(|h| h.1).or(selected.then_some(selection))
}

fn syntax_at(syntax: &[(Span, Rgb)], line: u64, col: usize) -> Option<Rgb> {
    syntax
        .iter()
        .rfind(|h| h.0.contains(line, col))
        .map(|h| h.1)
}

/// Итоговые цвета текста и фона с учётом INVERSE и DIM.
fn colors(cell: &Cell, look: &Appearance) -> (Rgb, Rgb) {
    let style = cell.style;
    let mut fg = resolve(style.fg, look.foreground, look);
    let mut bg = resolve(style.bg, look.background, look);
    if style.flags.contains(Flags::BOLD) && style.fg == Color::Default {
        fg = look.bold.unwrap_or(fg);
    }
    if style.flags.contains(Flags::INVERSE) {
        std::mem::swap(&mut fg, &mut bg);
    }
    if style.flags.contains(Flags::DIM) {
        fg = palette::dim(fg, bg);
    }
    if look.minimum_contrast > 1.0 && cell.ch != ' ' {
        fg = palette::ensure_contrast(fg, bg, look.minimum_contrast);
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
