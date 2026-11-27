//! Выделение мышью и работа с буфером обмена.

use std::time::{Duration, Instant};

use dimaster_config::CopyOnSelect;
use dimaster_grid::{Point, Selection, SelectionKind};
use dimaster_i18n::t;

use crate::clipboard::Target;
use crate::keys;
use crate::modes::{Mode, paste::PasteConfirm};
use crate::view::View;

/// Счётчик кликов и текущее выделение.
#[derive(Debug, Default)]
pub(crate) struct Selecting {
    pub(crate) selection: Option<Selection>,
    dragging: bool,
    drag_scroll_at: Option<Instant>,
    last_click: Option<(Instant, Point)>,
    clicks: u8,
}

impl Selecting {
    fn press(&mut self, point: Point, block: bool, now: Instant, window: Duration) {
        let repeated = self
            .last_click
            .is_some_and(|(at, last)| last == point && now.duration_since(at) < window);
        self.clicks = if repeated { self.clicks % 3 + 1 } else { 1 };
        self.last_click = Some((now, point));
        self.dragging = true;
        let kind = match self.clicks {
            2 => Some(SelectionKind::Word),
            3 => Some(SelectionKind::Line),
            _ if block => Some(SelectionKind::Block),
            // одиночный клик только снимает выделение, пока мышь не потянули
            _ => None,
        };
        self.selection = kind.map(|kind| Selection {
            kind,
            anchor: point,
            head: point,
        });
    }

    fn drag(&mut self, point: Point) -> bool {
        let Some((_, anchor)) = self.last_click.filter(|_| self.dragging) else {
            return false;
        };
        match &mut self.selection {
            Some(selection) => selection.head = point,
            None if point != anchor => {
                let kind = SelectionKind::Simple;
                self.selection = Some(Selection {
                    kind,
                    anchor,
                    head: point,
                });
            }
            None => return false,
        }
        true
    }

    pub(crate) fn clear(&mut self) {
        self.selection = None;
        self.dragging = false;
        self.drag_scroll_at = None;
    }

    pub(crate) fn is_dragging(&self) -> bool {
        self.dragging
    }
}

impl View {
    fn point(&self, (col, row): (usize, usize)) -> Point {
        Point {
            line: self.term.visible_line(self.scroll) + row as u64,
            col,
        }
    }

    pub(crate) fn select_press(&mut self) {
        let point = self.point(self.pointer.cell);
        self.selecting.press(
            point,
            self.modifiers.alt_key(),
            Instant::now(),
            Duration::from_millis(self.config.tuning.multi_click),
        );
        self.window.request_redraw();
    }

    pub(crate) fn select_drag(&mut self) {
        let point = self.point(self.pointer.cell);
        if self.selecting.drag(point) {
            self.window.request_redraw();
        }
    }

    /// Направление автопрокрутки, когда мышь вытянута выше или ниже сетки.
    pub(crate) fn drag_scroll_dir(&self) -> isize {
        let (_, y) = self.pointer.position;
        let top = f64::from(self.content_top() + self.config.padding.1);
        let bottom = f64::from(
            self.window
                .inner_size()
                .height
                .saturating_sub(self.content_bottom() + self.config.padding.1),
        );
        let cell = f64::from(self.fonts.metrics().cell_height.max(1));
        if y < top {
            (((top - y) / cell).ceil() as isize).clamp(1, 5)
        } else if y > bottom {
            -(((y - bottom) / cell).ceil() as isize).clamp(1, 5)
        } else {
            0
        }
    }

    /// Выполняет очередной шаг автопрокрутки при выделении за пределами окна.
    pub(crate) fn drag_scroll_step(&mut self) -> Option<Instant> {
        let dir = self.drag_scroll_dir();
        if !self.selecting.dragging || dir == 0 {
            self.selecting.drag_scroll_at = None;
            return None;
        }
        let now = Instant::now();
        let ready = self.selecting.drag_scroll_at.is_none_or(|at| now >= at);
        if ready {
            self.scroll_by(dir);
            self.select_drag();
            self.selecting.drag_scroll_at =
                Some(now + Duration::from_millis(self.config.tuning.drag_scroll_step));
        }
        self.selecting.drag_scroll_at
    }

    /// Отпускание кнопки кладёт выделенное в primary selection.
    pub(crate) fn select_release(&mut self) {
        self.selecting.drag_scroll_at = None;
        if std::mem::take(&mut self.selecting.dragging) {
            match self.config.copy_on_select {
                CopyOnSelect::Primary => self.copy(Target::Primary),
                CopyOnSelect::Clipboard => self.copy(Target::Clipboard),
                CopyOnSelect::Off => {}
            }
        }
    }

    pub(crate) fn copy(&mut self, target: Target) {
        let Some(selection) = self.selecting.selection else {
            return;
        };
        let text = self.term.selected_text(selection);
        if !text.is_empty() {
            self.put_clipboard(target, text);
        }
    }

    pub(crate) fn paste(&mut self, target: Target) {
        let Some(text) = self.clipboard.get(target) else {
            return;
        };
        let bracketed = self.term.modes().bracketed_paste;
        if self.config.clipboard_paste_protection
            && let Some(danger) = keys::paste_danger(&text, bracketed)
        {
            let reason = match (danger.controls, danger.lines > 1) {
                (true, true) => t!("{n} строк и управляющие символы", n = danger.lines),
                (true, false) => t!("управляющие символы"),
                (false, _) => t!("{n} строк без bracketed paste", n = danger.lines),
            };
            self.mode = Mode::Paste(PasteConfirm { text, reason });
            self.window.request_redraw();
            return;
        }
        self.type_bytes(keys::paste(&text, bracketed));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicks_cycle_through_kinds() {
        let (mut state, now) = (Selecting::default(), Instant::now());
        let point = Point { line: 0, col: 1 };
        let mut kinds = Vec::new();
        for _ in 0..4 {
            state.press(point, false, now, Duration::from_millis(400));
            kinds.push(state.selection.map(|s| s.kind));
        }
        let expected = [
            None,
            Some(SelectionKind::Word),
            Some(SelectionKind::Line),
            None,
        ];
        assert_eq!(kinds, expected);
    }

    #[test]
    fn drag_creates_selection_only_after_leaving_cell() {
        let mut state = Selecting::default();
        let start = Point { line: 0, col: 1 };
        state.press(start, false, Instant::now(), Duration::from_millis(400));
        assert!(!state.drag(start));
        assert!(state.drag(Point { line: 1, col: 0 }));
        assert_eq!(state.selection.unwrap().anchor, start);
    }
}
