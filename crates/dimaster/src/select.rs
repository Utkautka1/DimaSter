//! Выделение мышью и работа с буфером обмена.

use std::time::{Duration, Instant};

use dimaster_grid::{Point, Selection, SelectionKind};

use crate::clipboard::Target;
use crate::keys;
use crate::view::View;

// повторное нажатие в той же клетке за это время считается двойным
const MULTI_CLICK: Duration = Duration::from_millis(400);

/// Счётчик кликов и текущее выделение.
#[derive(Debug, Default)]
pub(crate) struct Selecting {
    pub(crate) selection: Option<Selection>,
    dragging: bool,
    last_click: Option<(Instant, Point)>,
    clicks: u8,
}

impl Selecting {
    fn press(&mut self, point: Point, block: bool, now: Instant) {
        let repeated = self
            .last_click
            .is_some_and(|(at, last)| last == point && now.duration_since(at) < MULTI_CLICK);
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
        self.selecting
            .press(point, self.modifiers.alt_key(), Instant::now());
        self.window.request_redraw();
    }

    pub(crate) fn select_drag(&mut self) {
        let point = self.point(self.pointer.cell);
        if self.selecting.drag(point) {
            self.window.request_redraw();
        }
    }

    /// Отпускание кнопки кладёт выделенное в primary selection.
    pub(crate) fn select_release(&mut self) {
        if std::mem::take(&mut self.selecting.dragging) {
            self.copy(Target::Primary);
        }
    }

    pub(crate) fn copy(&mut self, target: Target) {
        let Some(selection) = self.selecting.selection else {
            return;
        };
        let text = self.term.selected_text(selection);
        if !text.is_empty() {
            self.clipboard.set(target, text);
        }
    }

    pub(crate) fn paste(&mut self, target: Target) {
        if let Some(text) = self.clipboard.get(target) {
            let bytes = keys::paste(&text, self.term.modes().bracketed_paste);
            self.type_bytes(bytes);
        }
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
            state.press(point, false, now);
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
        state.press(start, false, Instant::now());
        assert!(!state.drag(start));
        assert!(state.drag(Point { line: 1, col: 0 }));
        assert_eq!(state.selection.unwrap().anchor, start);
    }
}
