//! Реакция окна на мышь: отчёты программе или прокрутка истории.

use winit::event::{ElementState, MouseButton};

use crate::mouse::{self, Action, Button, MouseEvent};
use crate::view::View;

/// Положение и состояние указателя.
#[derive(Debug, Default)]
pub(crate) struct Pointer {
    cell: (usize, usize),
    held: Option<Button>,
    // накопленная дробная часть прокрутки тачпада
    wheel_rest: f64,
}

impl View {
    pub(crate) fn pointer_moved(&mut self, x: f64, y: f64) {
        let cell = self.cell_at(x, y);
        if cell == self.pointer.cell {
            return;
        }
        self.pointer.cell = cell;
        let button = self.pointer.held.unwrap_or(Button::None);
        self.report(button, Action::Motion);
    }

    pub(crate) fn pointer_button(&mut self, state: ElementState, button: MouseButton) {
        let button = match button {
            MouseButton::Left => Button::Left,
            MouseButton::Middle => Button::Middle,
            MouseButton::Right => Button::Right,
            _ => return,
        };
        let pressed = state.is_pressed();
        self.pointer.held = pressed.then_some(button);
        self.report(
            button,
            if pressed {
                Action::Press
            } else {
                Action::Release
            },
        );
    }

    // Shift оставляет мышь терминалу, даже если программа её захватила
    fn report(&mut self, button: Button, action: Action) -> bool {
        if self.modifiers.shift_key() {
            return false;
        }
        let (col, row) = self.pointer.cell;
        let event = MouseEvent {
            button,
            action,
            col,
            row,
        };
        let Some(bytes) = mouse::encode(event, self.modifiers, self.term.modes()) else {
            return false;
        };
        self.send(bytes);
        true
    }

    /// `lines` > 0 — вверх, к истории.
    pub(crate) fn wheel(&mut self, lines: f64) {
        self.pointer.wheel_rest += lines;
        let whole = self.pointer.wheel_rest.trunc();
        self.pointer.wheel_rest -= whole;
        let steps = whole as isize;
        let button = if steps > 0 {
            Button::WheelUp
        } else {
            Button::WheelDown
        };
        for _ in 0..steps.unsigned_abs() {
            if !self.report(button, Action::Press) {
                return self.scroll_or_arrows(steps);
            }
        }
    }

    // в less и man истории нет — колесо превращается в стрелки
    fn scroll_or_arrows(&mut self, steps: isize) {
        if !self.term.is_alternate() {
            return self.scroll_by(steps);
        }
        let letter = if steps > 0 { 'A' } else { 'B' };
        let prefix = if self.term.modes().app_cursor {
            'O'
        } else {
            '['
        };
        let arrow = format!("\x1b{prefix}{letter}");
        self.send(arrow.repeat(steps.unsigned_abs()).into_bytes());
    }
}
