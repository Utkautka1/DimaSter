//! Полоса прокрутки с метками команд, ошибок и найденного.

use dimaster_config::Strip;
use dimaster_grid::PromptMark;
use dimaster_render::Rect;

use crate::modes::Mode;
use crate::view::View;

/// Область, по высоте которой раскладываются строки.
#[derive(Debug, Clone, Copy)]
struct Track {
    x: u32,
    width: u32,
    style: Strip,
    top: u32,
    height: u32,
    // строк всего вместе с историей
    total: u64,
}

impl Track {
    fn y(self, line: u64) -> u32 {
        self.top + (line * u64::from(self.height) / self.total.max(1)) as u32
    }

    fn rect(self, y: u32, height: u32, color: u32, translucent: bool) -> Rect {
        Rect {
            x: self.x,
            y,
            width: self.width,
            height,
            color,
            translucent,
            radius: 0,
            opacity: 1.0,
            stroke: None,
        }
    }

    // метка команды, ошибки или находки на своей строке
    fn mark(self, line: u64, color: u32) -> Rect {
        let height = dimaster_limits::SCROLLBAR_MARK_PX.u32();
        self.rect(self.y(line), height, color, false)
    }

    // ползунок: какая часть всех строк сейчас на экране
    fn thumb(self, first: u64, rows: u64) -> Rect {
        let top = self.y(first);
        let height = (self.y(first + rows) - top).max(dimaster_limits::SCROLLBAR_THUMB_PX.u32());
        let limit = self.top + self.height;
        self.rect(
            top.min(limit.saturating_sub(height)),
            height,
            self.style.color,
            true,
        )
    }
}

impl View {
    pub(crate) fn scrollbar_rects(&self) -> Vec<Rect> {
        let history = self.term.screen().history_len();
        if !self.config.scrollbar || self.term.is_alternate() || history == 0 {
            return Vec::new();
        }
        let (size, style) = (self.window.inner_size(), self.config.ui.scrollbar);
        let top = self.content_top();
        let range = self.term.line_range();
        let track = Track {
            x: match style.left {
                true => 1,
                false => size.width.saturating_sub(style.width + 1),
            },
            width: style.width,
            style,
            top,
            height: size.height.saturating_sub(top + self.content_bottom()),
            total: range.end - range.start,
        };
        let first = self.term.visible_line(self.scroll) - range.start;
        let mut rects = vec![track.thumb(first, self.term.screen().rows().len() as u64)];
        rects.extend(self.scroll_ticks(track, range));
        rects
    }

    // команды, ошибки и найденное
    fn scroll_ticks(&self, track: Track, range: std::ops::Range<u64>) -> Vec<Rect> {
        let tick = |line: u64, color| track.mark(line - range.start, color);
        let mut ticks: Vec<Rect> = range
            .clone()
            .filter_map(|line| {
                let color = match self.term.line(line)?.mark? {
                    PromptMark::Prompt => track.style.command,
                    PromptMark::Failed => track.style.error,
                    PromptMark::Output => return None,
                };
                Some(tick(line, color))
            })
            .collect();
        if let Mode::Search(search) = &self.mode {
            ticks.extend(
                search
                    .matches
                    .iter()
                    .map(|m| tick(m.start.line, track.style.found)),
            );
        }
        ticks
    }
}
