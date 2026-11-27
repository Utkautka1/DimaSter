//! Отрисовка миникарты и щелчки по ней.

use dimaster_config::Strip;
use dimaster_grid::PromptMark;
use dimaster_render::Rect;

use crate::modes::Mode;
use crate::view::View;

/// Где нарисована карта и сколько строк в одном пикселе.
#[derive(Debug, Clone, Copy)]
struct Area {
    x: u32,
    style: Strip,
    top: u32,
    height: u32,
    total: u64,
}

impl Area {
    // строк на одну полоску в 2 пикселя; короткая история не растягивается
    fn step(self) -> u64 {
        (self.total * dimaster_limits::MINIMAP_ROW_PX.get())
            .div_ceil(u64::from(self.height).max(1))
            .max(1)
    }

    fn y(self, line: u64) -> u32 {
        self.top + (line / self.step()) as u32 * dimaster_limits::MINIMAP_ROW_PX.u32()
    }

    fn line_at(self, y: f64) -> u64 {
        let offset =
            (y - f64::from(self.top)).max(0.0) as u64 / dimaster_limits::MINIMAP_ROW_PX.get();
        (offset * self.step()).min(self.total.saturating_sub(1))
    }
}

impl View {
    fn minimap_area(&self) -> Option<Area> {
        let shown = self.config.widgets.minimap || matches!(self.mode, Mode::Minimap(_));
        if !shown || self.term.is_alternate() {
            return None;
        }
        let (size, style) = (self.window.inner_size(), self.config.ui.minimap);
        let top = self.content_top();
        let range = self.term.line_range();
        Some(Area {
            x: match style.left {
                true => self.config.tuning.minimap_margin,
                false => size
                    .width
                    .saturating_sub(style.width + self.config.tuning.minimap_margin),
            },
            style,
            top,
            height: size.height.saturating_sub(top + self.content_bottom()),
            total: range.end - range.start,
        })
    }

    // одна полоска на группу строк: длина по самой длинной, цвет по метке
    fn minimap_bar(&self, area: Area, first: u64) -> Rect {
        let start = self.term.line_range().start;
        let mut longest = 0;
        let (style, mut color) = (area.style, area.style.color);
        for line in first..(first + area.step()).min(area.total) {
            let Some(row) = self.term.line(start + line) else {
                continue;
            };
            longest = longest.max(row.text().chars().count() as u32);
            match row.mark {
                Some(PromptMark::Failed) => color = style.error,
                Some(PromptMark::Prompt) if color != style.error => color = style.command,
                _ => {}
            }
        }
        let width = (longest * 2 / 3).min(style.width);
        self.plain_rect(area.x, area.y(first), (width, 1), color)
    }

    // фон карты и рамка видимой части
    fn minimap_frame(&self, area: Area) -> [Rect; 2] {
        let first = self.term.visible_line(self.scroll) - self.term.line_range().start;
        let rows = self.term.screen().rows().len() as u64;
        let size = (
            area.style.width + dimaster_limits::MINIMAP_VIEW_PX.u32(),
            area.height,
        );
        let x = area
            .x
            .saturating_sub(dimaster_limits::MINIMAP_VIEW_PX.u32());
        let backdrop = self.plain_rect(x, area.top, size, self.config.theme.background);
        let view = Rect {
            y: area.y(first),
            height: (area.y(first + rows) - area.y(first))
                .max(dimaster_limits::MINIMAP_VIEW_PX.u32()),
            color: area.style.color,
            translucent: true,
            ..backdrop
        };
        [backdrop, view]
    }

    pub(crate) fn minimap_rects(&self) -> Vec<Rect> {
        let Some(area) = self.minimap_area() else {
            return Vec::new();
        };
        let bars = (0..area.total).step_by(area.step() as usize);
        let mut rects = self.minimap_frame(area).to_vec();
        rects.extend(bars.map(|line| self.minimap_bar(area, line)));
        rects
    }

    /// Щелчок по карте переносит к этому месту; `true` — щелчок принят.
    pub(crate) fn minimap_click(&mut self) -> bool {
        let (x, y) = self.pointer.position;
        let Some(area) = self
            .minimap_area()
            .filter(|a| x >= f64::from(a.x) && x < f64::from(a.x + a.style.width))
        else {
            return false;
        };
        let line = self.term.line_range().start + area.line_at(y);
        self.reveal(line);
        self.window.request_redraw();
        true
    }
}
