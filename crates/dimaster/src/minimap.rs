//! Отрисовка миникарты и щелчки по ней.

use dimaster_grid::PromptMark;
use dimaster_render::Rect;

use crate::modes::Mode;
use crate::view::View;

const WIDTH: u32 = 90;
// отступ от полосы прокрутки
const MARGIN: u32 = 8;
const TEXT: u32 = 0x9aa5ce;
const COMMAND: u32 = 0x7aa2f7;
const ERROR: u32 = 0xf7768e;

/// Где нарисована карта и сколько строк в одном пикселе.
#[derive(Debug, Clone, Copy)]
struct Area {
    x: u32,
    top: u32,
    height: u32,
    total: u64,
}

impl Area {
    // строк на одну полоску в 2 пикселя; короткая история не растягивается
    fn step(self) -> u64 {
        (self.total * 2)
            .div_ceil(u64::from(self.height).max(1))
            .max(1)
    }

    fn y(self, line: u64) -> u32 {
        self.top + (line / self.step()) as u32 * 2
    }

    fn line_at(self, y: f64) -> u64 {
        let offset = (y - f64::from(self.top)).max(0.0) as u64 / 2;
        (offset * self.step()).min(self.total.saturating_sub(1))
    }
}

impl View {
    fn minimap_area(&self) -> Option<Area> {
        let shown = self.config.widgets.minimap || matches!(self.mode, Mode::Minimap(_));
        if !shown || self.term.is_alternate() {
            return None;
        }
        let size = self.window.inner_size();
        let top = self.title_height();
        let range = self.term.line_range();
        Some(Area {
            x: size.width.saturating_sub(WIDTH + MARGIN),
            top,
            height: size.height.saturating_sub(top + self.status_height()),
            total: range.end - range.start,
        })
    }

    // одна полоска на группу строк: длина по самой длинной, цвет по метке
    fn minimap_bar(&self, area: Area, first: u64) -> Rect {
        let start = self.term.line_range().start;
        let mut longest = 0;
        let mut color = TEXT;
        for line in first..(first + area.step()).min(area.total) {
            let Some(row) = self.term.line(start + line) else {
                continue;
            };
            longest = longest.max(row.text().chars().count() as u32);
            match row.mark {
                Some(PromptMark::Failed) => color = ERROR,
                Some(PromptMark::Prompt) if color != ERROR => color = COMMAND,
                _ => {}
            }
        }
        let width = (longest * 2 / 3).min(WIDTH);
        Rect {
            x: area.x,
            y: area.y(first),
            width,
            height: 1,
            color,
            translucent: false,
        }
    }

    pub(crate) fn minimap_rects(&self) -> Vec<Rect> {
        let Some(area) = self.minimap_area() else {
            return Vec::new();
        };
        let first = self.term.visible_line(self.scroll) - self.term.line_range().start;
        let rows = self.term.screen().rows().len() as u64;
        let backdrop = Rect {
            x: area.x - 4,
            y: area.top,
            width: WIDTH + 4,
            height: area.height,
            color: self.config.theme.background,
            translucent: false,
        };
        // рамка видимой части
        let height = (area.y(first + rows) - area.y(first)).max(4);
        let view = Rect {
            y: area.y(first),
            height,
            color: TEXT,
            translucent: true,
            ..backdrop
        };
        let bars = (0..area.total).step_by(area.step() as usize);
        let mut rects = vec![backdrop, view];
        rects.extend(bars.map(|line| self.minimap_bar(area, line)));
        rects
    }

    /// Щелчок по карте переносит к этому месту; `true` — щелчок принят.
    pub(crate) fn minimap_click(&mut self) -> bool {
        let (x, y) = self.pointer.position;
        let Some(area) = self.minimap_area().filter(|a| x >= f64::from(a.x)) else {
            return false;
        };
        let line = self.term.line_range().start + area.line_at(y);
        self.reveal(line);
        self.window.request_redraw();
        true
    }
}
