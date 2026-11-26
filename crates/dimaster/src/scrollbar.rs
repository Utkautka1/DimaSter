//! Полоса прокрутки с метками команд, ошибок и найденного.

use dimaster_grid::PromptMark;
use dimaster_render::Rect;

use crate::modes::Mode;
use crate::view::View;

const WIDTH: u32 = 5;
const THUMB: u32 = 0x9aa5ce;
const COMMAND: u32 = 0x7aa2f7;
const ERROR: u32 = 0xf7768e;
const FOUND: u32 = 0xe0af68;

/// Область, по высоте которой раскладываются строки.
#[derive(Debug, Clone, Copy)]
struct Track {
    x: u32,
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
            width: WIDTH,
            height,
            color,
            translucent,
        }
    }

    // ползунок: какая часть всех строк сейчас на экране
    fn thumb(self, first: u64, rows: u64) -> Rect {
        let top = self.y(first);
        let height = (self.y(first + rows) - top).max(12);
        let limit = self.top + self.height;
        self.rect(top.min(limit.saturating_sub(height)), height, THUMB, true)
    }
}

impl View {
    pub(crate) fn scrollbar_rects(&self) -> Vec<Rect> {
        let history = self.term.screen().history().count();
        if !self.config.scrollbar || self.term.is_alternate() || history == 0 {
            return Vec::new();
        }
        let size = self.window.inner_size();
        let top = self.title_height();
        let range = self.term.line_range();
        let track = Track {
            x: size.width.saturating_sub(WIDTH + 1),
            top,
            height: size.height.saturating_sub(top + self.status_height()),
            total: range.end - range.start,
        };
        let first = self.term.visible_line(self.scroll) - range.start;
        let mut rects = vec![track.thumb(first, self.term.screen().rows().len() as u64)];
        rects.extend(self.scroll_ticks(track, range));
        rects
    }

    // команды, ошибки и найденное
    fn scroll_ticks(&self, track: Track, range: std::ops::Range<u64>) -> Vec<Rect> {
        let tick = |line: u64, color| track.rect(track.y(line - range.start), 2, color, false);
        let mut ticks: Vec<Rect> = range
            .clone()
            .filter_map(|line| {
                let color = match self.term.line(line)?.mark? {
                    PromptMark::Prompt => COMMAND,
                    PromptMark::Failed => ERROR,
                    PromptMark::Output => return None,
                };
                Some(tick(line, color))
            })
            .collect();
        if let Mode::Search(search) = &self.mode {
            ticks.extend(search.matches.iter().map(|m| tick(m.start.line, FOUND)));
        }
        ticks
    }
}
