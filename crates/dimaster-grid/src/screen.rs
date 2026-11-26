//! Буфер видимых строк с областью прокрутки и историей.

use std::collections::VecDeque;

use crate::{Cell, Row, Size};

/// Экран (основной или альтернативный).
#[derive(Debug)]
pub struct Screen {
    pub(crate) rows: Vec<Row>,
    /// Строки, ушедшие вверх; у альтернативного экрана всегда пусто.
    pub(crate) history: VecDeque<Row>,
    // сколько строк уже вытеснено из истории: делает номера строк постоянными
    pub(crate) evicted: u64,
    // максимум строк истории; 0 отключает её
    pub(crate) limit: usize,
    // область прокрутки, обе границы включительно
    pub(crate) top: usize,
    pub(crate) bottom: usize,
}

impl Screen {
    pub(crate) fn new(size: Size, limit: usize) -> Self {
        let rows = vec![Row::new(size.cols, &Cell::default()); size.rows];
        let history = VecDeque::new();
        Self {
            rows,
            history,
            evicted: 0,
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
    pub(crate) fn remember(&mut self, row: &mut Row) {
        if self.limit == 0 {
            return;
        }
        let mut cells = self.recycle_oldest();
        let blank = Cell::default();
        // у перенесённой строки хвостовые пробелы — часть текста
        let used = match row.wrapped {
            true => row.cells.len(),
            false => row
                .cells
                .iter()
                .rposition(|c| *c != blank)
                .map_or(0, |i| i + 1),
        };
        cells.clear();
        cells.extend(row.cells.drain(..used));
        self.history.push_back(Row {
            cells,
            wrapped: row.wrapped,
            mark: row.mark,
        });
    }

    // заполненная история отдаёт буфер самой старой строки новой
    fn recycle_oldest(&mut self) -> Vec<Cell> {
        if self.history.len() < self.limit {
            return Vec::new();
        }
        self.evicted += 1;
        self.history
            .pop_front()
            .map(|old| old.cells)
            .unwrap_or_default()
    }

    pub(crate) fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        let excess = self.history.len().saturating_sub(limit);
        self.history.drain(..excess);
        self.evicted += excess as u64;
    }

    /// Меняет размер без переразбиения: так ведёт себя альтернативный экран,
    /// который программа всё равно перерисует сама.
    pub(crate) fn resize(&mut self, size: Size) {
        self.rows
            .resize(size.rows, Row::new(size.cols, &Cell::default()));
        self.rows.iter_mut().for_each(|r| r.resize(size.cols));
        self.top = 0;
        self.bottom = size.rows - 1;
    }
}
