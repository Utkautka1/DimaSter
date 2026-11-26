//! Буфер видимых строк с областью прокрутки и историей.

use std::collections::VecDeque;

use crate::history::PackedRow;
use crate::{Cell, Row, Size};

/// Экран (основной или альтернативный).
#[derive(Debug)]
pub struct Screen {
    pub(crate) rows: Vec<Row>,
    /// Строки, ушедшие вверх в сжатом виде; у альтернативного экрана всегда пусто.
    pub(crate) history: VecDeque<PackedRow>,
    // сколько строк уже вытеснено из истории: делает номера строк постоянными
    pub(crate) evicted: u64,
    // максимум строк истории; 0 отключает её
    pub(crate) limit: usize,
    // false — картинки не переживают уход строки в историю
    pub(crate) keep_images: bool,
    // область прокрутки, обе границы включительно
    pub(crate) top: usize,
    pub(crate) bottom: usize,
    // боковые поля, включительно
    pub(crate) left: usize,
    pub(crate) right: usize,
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
            keep_images: true,
            top: 0,
            bottom: size.rows - 1,
            left: 0,
            right: size.cols - 1,
        }
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Количество строк в сжатой истории за O(1).
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Строки истории с распаковкой по мере обхода.
    pub fn history(&self) -> impl DoubleEndedIterator<Item = Row> + '_ {
        self.history.iter().map(PackedRow::unpack)
    }

    /// Оценочный объём памяти, занятый историей (в байтах).
    pub fn history_bytes(&self) -> usize {
        self.history.iter().map(PackedRow::heap_bytes).sum()
    }

    pub(crate) fn cols(&self) -> usize {
        self.rows[0].cells.len()
    }

    /// Прокручивает область вверх; в историю уходят только строки с самого верха.
    ///
    /// Буфер ушедшей строки переиспользуется как новая пустая: прокрутка —
    /// самая частая операция, и выделять на неё всю ширину слишком дорого.
    pub(crate) fn scroll_up(&mut self, count: usize, fill: &Cell) {
        if self.narrowed() {
            return self.scroll_span(count, true, fill);
        }
        let count = count.min(self.bottom - self.top + 1);
        let cols = self.cols();
        for _ in 0..count {
            self.rows[self.top..=self.bottom].rotate_left(1);
            let mut row = std::mem::replace(&mut self.rows[self.bottom], Row::empty());
            let keep = self.top == 0 && self.limit > 0 && row.cells.len() == cols;
            if self.top == 0 {
                self.remember(&mut row);
            }
            if keep && *fill == Cell::default() {
                row.cells.fill(Cell::default());
                row.recycle();
            } else {
                row.reset(cols, fill);
            }
            self.rows[self.bottom] = row;
        }
    }

    pub(crate) fn scroll_down(&mut self, count: usize, fill: &Cell) {
        if self.narrowed() {
            return self.scroll_span(count, false, fill);
        }
        let count = count.min(self.bottom - self.top + 1);
        let blank = Row::new(self.cols(), fill);
        for _ in 0..count {
            self.rows.remove(self.bottom);
            self.rows.insert(self.top, blank.clone());
        }
    }

    // В истории храним сжатое представление строки: текст в UTF-8 и прогоны стилей
    pub(crate) fn remember(&mut self, row: &mut Row) {
        if self.limit == 0 {
            return;
        }
        if self.history.len() >= self.limit {
            self.evicted += 1;
            self.history.pop_front();
        }
        let images = match self.keep_images {
            true => std::mem::take(&mut row.images),
            false => Vec::new(),
        };
        let packed = PackedRow::pack(&mut row.cells, row.wrapped, row.mark, images);
        self.history.push_back(packed);
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
