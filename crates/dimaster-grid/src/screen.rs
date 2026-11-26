//! Буфер видимых строк с областью прокрутки и историей.

use std::collections::VecDeque;

use crate::{Cell, Row, Size};

/// Экран (основной или альтернативный).
#[derive(Debug)]
pub struct Screen {
    pub(crate) rows: Vec<Row>,
    /// Строки, ушедшие вверх; у альтернативного экрана всегда пусто.
    pub(crate) history: VecDeque<Row>,
    // максимум строк истории; 0 отключает её
    limit: usize,
    // область прокрутки, обе границы включительно
    pub(crate) top: usize,
    pub(crate) bottom: usize,
}

impl Screen {
    pub(crate) fn new(size: Size, limit: usize) -> Self {
        let rows = vec![Row::new(size.cols, &Cell::default()); size.rows];
        Self {
            rows,
            history: VecDeque::new(),
            limit,
            top: 0,
            bottom: size.rows - 1,
        }
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn history(&self) -> impl DoubleEndedIterator<Item = &Row> {
        self.history.iter()
    }

    pub(crate) fn cols(&self) -> usize {
        self.rows[0].cells.len()
    }

    /// Прокручивает область вверх; в историю уходят только строки с самого верха.
    ///
    /// Буфер ушедшей строки переиспользуется как новая пустая: прокрутка —
    /// самая частая операция, и выделять на неё всю ширину слишком дорого.
    pub(crate) fn scroll_up(&mut self, count: usize, fill: &Cell) {
        let count = count.min(self.bottom - self.top + 1);
        let cols = self.cols();
        for _ in 0..count {
            self.rows[self.top..=self.bottom].rotate_left(1);
            let mut row = std::mem::replace(&mut self.rows[self.bottom], Row::empty());
            if self.top == 0 {
                self.remember(&mut row);
            }
            row.reset(cols, fill);
            self.rows[self.bottom] = row;
        }
    }

    pub(crate) fn scroll_down(&mut self, count: usize, fill: &Cell) {
        let count = count.min(self.bottom - self.top + 1);
        let blank = Row::new(self.cols(), fill);
        for _ in 0..count {
            self.rows.remove(self.bottom);
            self.rows.insert(self.top, blank.clone());
        }
    }

    // В истории храним только занятую часть строки — это в разы меньше памяти
    fn remember(&mut self, row: &mut Row) {
        if self.limit == 0 {
            return;
        }
        let mut cells = match self.history.len() == self.limit {
            true => self
                .history
                .pop_front()
                .map(|old| old.cells)
                .unwrap_or_default(),
            false => Vec::new(),
        };
        let blank = Cell::default();
        let used = row
            .cells
            .iter()
            .rposition(|c| *c != blank)
            .map_or(0, |i| i + 1);
        cells.clear();
        cells.extend(row.cells.drain(..used));
        self.history.push_back(Row {
            cells,
            wrapped: row.wrapped,
            mark: row.mark,
        });
    }

    /// Меняет размер без переразбиения строк (reflow появится позже).
    /// Возвращает, на сколько строк сместилось содержимое вверх.
    pub(crate) fn resize(&mut self, size: Size, cursor_row: usize) -> usize {
        // при уменьшении высоты уводим верх в историю, чтобы курсор остался виден
        // сохранённый курсор мог остаться ниже экрана после прошлых resize
        let cursor_row = cursor_row.min(self.rows.len() - 1);
        let shift = (cursor_row + 1).saturating_sub(size.rows);
        for mut row in self.rows.drain(..shift).collect::<Vec<_>>() {
            self.remember(&mut row);
        }
        self.rows
            .resize(size.rows, Row::new(size.cols, &Cell::default()));
        self.rows.iter_mut().for_each(|r| r.resize(size.cols));
        self.top = 0;
        self.bottom = size.rows - 1;
        shift
    }
}
