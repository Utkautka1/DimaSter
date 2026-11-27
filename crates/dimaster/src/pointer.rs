//! Реакция окна на мышь: отчёты программе или прокрутка истории.

use winit::event::{ElementState, MouseButton};

use dimaster_grid::{Match, Point};

use crate::clipboard::Target;
use crate::desktop;
use crate::mouse::{self, Action, Button, MouseEvent};
use crate::view::View;

/// Положение и состояние указателя.
#[derive(Debug, Default)]
pub(crate) struct Pointer {
    pub(crate) cell: (usize, usize),
    // в пикселях окна
    pub(crate) position: (f64, f64),
    held: Option<Button>,
    // накопленная дробная часть прокрутки тачпада
    wheel_rest: f64,
}

impl View {
    pub(crate) fn pointer_moved(&mut self, x: f64, y: f64) {
        if self.config.mouse_hide_while_typing {
            self.window.set_cursor_visible(true);
        }
        self.pointer.position = (x, y);
        let cell = self.cell_at(x, y);
        self.update_hover(cell);
        self.update_pointer_icon();
        if self.selecting.is_dragging() && self.drag_scroll_dir() != 0 {
            self.drag_scroll_step();
        }
        if cell == self.pointer.cell {
            return;
        }
        self.pointer.cell = cell;
        let button = self.pointer.held.unwrap_or(Button::None);
        if !self.report(button, Action::Motion) {
            self.select_drag();
        }
    }

    pub(crate) fn pointer_button(&mut self, state: ElementState, button: MouseButton) {
        let button = match button {
            MouseButton::Left => Button::Left,
            MouseButton::Middle => Button::Middle,
            MouseButton::Right => Button::Right,
            _ => return,
        };
        let pressed = state.is_pressed();
        if pressed
            && button == Button::Left
            && (self.chrome_press() || self.minimap_click() || self.follow_link())
        {
            return;
        }
        self.pointer.held = pressed.then_some(button);
        let action = if pressed {
            Action::Press
        } else {
            Action::Release
        };
        if !self.report(button, action) {
            self.local_button(button, action);
        }
    }

    fn update_hover(&mut self, (col, row): (usize, usize)) {
        let line = self.term.visible_line(self.scroll) + row as u64;
        let link = self
            .modifiers
            .control_key()
            .then(|| self.link_under(Point { line, col }))
            .flatten();
        // адрес гиперссылки может отличаться от видимого текста — показываем его
        self.hover_uri = link.as_ref().map(|(_, uri)| uri.clone());
        let link = link.map(|(range, _)| range);
        if link != self.hover_link {
            self.hover_link = link;
            self.window.request_redraw();
        }
    }

    // существующий файл или папка под указателем важнее похожего на домен
    fn link_under(&self, point: Point) -> Option<(Match, String)> {
        let cwd = self.term.cwd();
        let local = self.term.word_at(point).filter(|(_, word)| {
            desktop::target(word, cwd).is_some_and(|t| t.is_absolute() || t.exists())
        });
        local.or_else(|| self.term.link_at(point))
    }

    // Ctrl+клик открывает ссылку или файл
    fn follow_link(&mut self) -> bool {
        let (col, row) = self.pointer.cell;
        let line = self.term.visible_line(self.scroll) + row as u64;
        let found = self
            .modifiers
            .control_key()
            .then(|| self.link_under(Point { line, col }));
        let Some((_, link)) = found.flatten() else {
            return false;
        };
        desktop::open(&link, self.term.cwd());
        true
    }

    // мышь не нужна программе — значит, это выделение или вставка
    fn local_button(&mut self, button: Button, action: Action) {
        match (button, action) {
            (Button::Left, Action::Press) => self.select_press(),
            (Button::Left, Action::Release) => self.select_release(),
            (Button::Middle, Action::Press) => self.paste(Target::Primary),
            _ => {}
        }
    }

    // Shift оставляет мышь терминалу, даже если программа её захватила
    fn report(&mut self, button: Button, action: Action) -> bool {
        // Shift обычно оставляет мышь терминалу; программа может просить его себе
        let wanted = self.config.programs.shift_mouse && self.term.modes().shift_escape;
        if self.modifiers.shift_key() && !wanted {
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
        // `lines` — щелчки колеса; множитель из настроек
        self.pointer.wheel_rest += lines * f64::from(self.config.scroll_multiplier);
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
                self.scroll_or_arrows(steps);
                // выделение тянется вслед за прокруткой
                return self.select_drag();
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
