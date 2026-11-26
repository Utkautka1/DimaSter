//! Сборка кадра: подсветки, панели и положение курсора.

use std::time::Instant;

use dimaster_grid::{Point, Span};
use dimaster_render::{Label, Viewport};

use crate::chrome::BUTTONS;
use crate::modes::Mode;
use crate::view::View;

const MATCH: u32 = 0x6b5a1e;
const CURRENT_MATCH: u32 = 0xb0620f;
const LINK: u32 = 0x2f4a7a;
// строк палитры на экране
const PALETTE_ROWS: usize = 11;

// доля `amount` второго цвета в смеси
fn blend(base: u32, other: u32, amount: f32) -> u32 {
    let channel = |shift: u32| {
        let (a, b) = (
            ((base >> shift) & 0xff) as f32,
            ((other >> shift) & 0xff) as f32,
        );
        ((a + (b - a) * amount).round() as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

impl View {
    fn highlights(&self) -> Vec<(Span, u32)> {
        let mut out = Vec::new();
        if let Mode::Search(search) = &self.mode {
            let first = self.term.visible_line(self.scroll);
            let last = first + self.term.screen().rows().len() as u64;
            for (index, found) in search.matches.iter().enumerate() {
                if found.end.line >= first && found.start.line < last {
                    let color = if index == search.current {
                        CURRENT_MATCH
                    } else {
                        MATCH
                    };
                    out.push((self.term.match_span(*found), color));
                }
            }
        }
        out.extend(self.hover_link.map(|l| (self.term.match_span(l), LINK)));
        out
    }

    // цвета панелей выводятся из фона и текста темы: цвет выделения бывает
    // светлым, и светлый текст на нём не читается
    pub(crate) fn label(&self, x: u32, y: u32, width: usize, text: String) -> Label {
        let theme = self.config.theme;
        let background = blend(theme.background, theme.foreground, 0.14);
        Label {
            x,
            y,
            width,
            text,
            color: theme.foreground,
            background,
        }
    }

    fn title_labels(&self) -> Vec<Label> {
        if self.title_height() == 0 {
            return Vec::new();
        }
        let cell = self.fonts.metrics().cell_width;
        let width = (self.window.inner_size().width / cell) as usize;
        let title = self.window.title();
        let room = width.saturating_sub(BUTTONS.chars().count() + 2);
        let mut bar = self.label(cell, 4, 0, title.chars().take(room).collect());
        bar.background = self.config.theme.background;
        let buttons_x = self.window.inner_size().width.saturating_sub(cell * 9);
        let buttons = Label {
            x: buttons_x,
            text: BUTTONS.to_owned(),
            ..bar.clone()
        };
        vec![bar, buttons]
    }

    // нижняя строка: поиск или подсказка vi-режима
    fn status_label(&self) -> Option<Label> {
        let text = match &self.mode {
            Mode::Search(search) => search.status(),
            Mode::Hint(_) => " Метка — открыть, ЗАГЛАВНАЯ — копировать, Esc ".into(),
            Mode::Vi(_) => " VI: hjkl w b 0 $ g G · v V · y копировать · / поиск · q выход ".into(),
            _ => return None,
        };
        let metrics = self.fonts.metrics();
        let size = self.window.inner_size();
        let width = (size.width / metrics.cell_width) as usize;
        Some(self.label(
            0,
            size.height.saturating_sub(metrics.cell_height),
            width,
            text,
        ))
    }

    fn palette_labels(&self) -> Vec<Label> {
        let Mode::Palette(palette) = &self.mode else {
            return Vec::new();
        };
        let metrics = self.fonts.metrics();
        let columns = (self.window.inner_size().width / metrics.cell_width) as usize;
        let width = columns.min(50);
        let x = (columns - width) as u32 / 2 * metrics.cell_width;
        let top = self.title_height() + metrics.cell_height;
        let row = |index: usize, text: String| {
            self.label(x, top + index as u32 * metrics.cell_height, width, text)
        };
        let mut labels = vec![row(0, format!(" > {}_", palette.query))];
        for (index, (name, _)) in palette.items().into_iter().take(PALETTE_ROWS).enumerate() {
            let mut label = row(index + 1, format!("   {name}"));
            if index == palette.selected {
                (label.background, label.color) = (label.color, self.config.theme.background);
            }
            labels.push(label);
        }
        labels
    }

    // (строка, колонка) на экране; None — курсор за пределами вида
    fn screen_position(&self, point: Point) -> Option<(f32, f32)> {
        let row = point
            .line
            .checked_sub(self.term.visible_line(self.scroll))?;
        (row < self.term.screen().rows().len() as u64).then_some((row as f32, point.col as f32))
    }

    fn cursor_target(&self) -> Option<(f32, f32)> {
        if let Mode::Vi(vi) = &self.mode {
            return self.screen_position(vi.cursor);
        }
        let cursor = self.term.cursor();
        let visible = self.scroll == 0 && self.term.modes().cursor_visible;
        visible.then_some((cursor.row as f32, cursor.col as f32))
    }

    // выделение и панели зависят от режима
    fn decorations(&self) -> (Option<Span>, Vec<(Span, u32)>, Vec<Label>) {
        let selection = match &self.mode {
            Mode::Vi(vi) => vi.selection(),
            _ => self.selecting.selection,
        };
        let mut labels = self.title_labels();
        labels.extend(self.status_label());
        labels.extend(self.palette_labels());
        labels.extend(self.mode_panels());
        (
            selection.map(|s| self.term.span(s)),
            self.highlights(),
            labels,
        )
    }

    pub(crate) fn draw(&mut self) {
        let target = self.cursor_target();
        let (effects, now) = (self.config.effects, Instant::now());
        let pose = self
            .animator
            .tick(target.unwrap_or_default(), effects, self.focused, now);
        let (selection, highlights, labels) = self.decorations();
        let mut rects = self.scrollbar_rects();
        rects.extend(self.glow_rects());
        let view = Viewport {
            scroll: self.scroll,
            selection,
            preedit: &self.preedit,
            highlights: &highlights,
            labels: &labels,
            offset: pose.offset,
            cursor: target.map(|_| pose.cursor),
            cursor_hidden: target.is_none() || pose.blink_hidden,
            trail: if target.is_some() { &pose.trail } else { &[] },
            rects: &rects,
        };
        if !self.renderer.draw(&self.term, &mut self.fonts, view) {
            (self.wake)();
        }
        self.schedule_animation();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blends_towards_second_color() {
        assert_eq!(blend(0x000000, 0xffffff, 0.5), 0x808080);
        assert_eq!(blend(0x102030, 0x102030, 0.3), 0x102030);
    }
}
