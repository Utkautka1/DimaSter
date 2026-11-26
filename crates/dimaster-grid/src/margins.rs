//! Левое и правое поля (DECLRMM, DECSLRM) и всё, что от них зависит.

use crate::terminal::State;
use crate::{Cell, Screen};

impl Screen {
    /// Поля уже экрана — прокручивается только часть строки.
    pub(crate) fn narrowed(&self) -> bool {
        self.left > 0 || self.right + 1 < self.cols()
    }

    pub(crate) fn reset_margins(&mut self) {
        self.left = 0;
        self.right = self.cols() - 1;
    }

    // переносит колонки полей из строки `from` в `to`; `None` — заливка
    fn move_span(&mut self, from: Option<usize>, to: usize, fill: &Cell) {
        let (left, right) = (self.left, self.right);
        let span: Vec<Cell> = match from {
            Some(row) => self.rows[row].cells[left..=right].to_vec(),
            None => vec![fill.clone(); right - left + 1],
        };
        self.rows[to].cells[left..=right].clone_from_slice(&span);
        self.rows[to].wrapped = false;
    }

    /// Прокрутка внутри полей в строках `top..=bottom`; в историю не уходит.
    pub(crate) fn scroll_span(&mut self, count: usize, up: bool, fill: &Cell) {
        let (top, bottom) = (self.top, self.bottom);
        let count = count.min(bottom - top + 1);
        for step in 0..=bottom - top {
            let to = if up { top + step } else { bottom - step };
            let from = match up {
                true => Some(to + count).filter(|&r| r <= bottom),
                false => to.checked_sub(count).filter(|&r| r >= top),
            };
            self.move_span(from, to, fill);
        }
    }
}

impl State {
    pub(crate) fn within_margins(&self) -> bool {
        (self.screen.left..=self.screen.right).contains(&self.cursor.col)
    }

    /// Колонка, за которую печать и курсор не выходят.
    pub(crate) fn right_edge(&self) -> usize {
        match self.cursor.col <= self.screen.right {
            true => self.screen.right,
            false => self.screen.cols() - 1,
        }
    }

    pub(crate) fn left_edge(&self) -> usize {
        match self.cursor.col >= self.screen.left {
            true => self.screen.left,
            false => 0,
        }
    }

    pub(crate) fn carriage_return(&mut self) {
        let column = match self.modes.origin {
            true => self.screen.left,
            false => self.left_edge(),
        };
        self.set_col(column);
    }

    /// DECSET 69: без режима поля сбрасываются.
    pub(crate) fn set_margin_mode(&mut self, on: bool) {
        self.modes.margins = on;
        if !on {
            self.screen.reset_margins();
        }
    }

    /// DECSLRM, колонки с нуля; поля уже двух колонок не принимаются.
    pub(crate) fn set_margins(&mut self, left: usize, right: usize) {
        let right = right.min(self.screen.cols() - 1);
        if left < right {
            (self.screen.left, self.screen.right) = (left, right);
            self.goto(0, 0);
        }
    }
}
