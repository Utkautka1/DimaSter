//! Операции, меняющие содержимое и положение курсора.

use crate::terminal::State;
use crate::{Cell, Flags, Row};

impl State {
    pub(crate) fn blank(&self) -> Cell {
        Cell::blank(self.cursor.style)
    }

    pub(crate) fn line_feed(&mut self) {
        self.cursor.pending_wrap = false;
        if self.cursor.row == self.screen.bottom {
            let fill = self.blank();
            self.screen.scroll_up(1, &fill);
        } else if self.cursor.row + 1 < self.screen.rows.len() {
            self.cursor.row += 1;
        }
    }

    pub(crate) fn reverse_line_feed(&mut self) {
        self.cursor.pending_wrap = false;
        if self.cursor.row == self.screen.top {
            let fill = self.blank();
            self.screen.scroll_down(1, &fill);
        } else {
            self.cursor.row = self.cursor.row.saturating_sub(1);
        }
    }

    /// Абсолютное перемещение; в origin mode строки считаются от области прокрутки.
    pub(crate) fn goto(&mut self, row: usize, col: usize) {
        let (base, last) = if self.modes.origin {
            (self.screen.top, self.screen.bottom)
        } else {
            (0, self.screen.rows.len() - 1)
        };
        self.cursor.row = (base + row).min(last);
        self.set_col(col);
    }

    pub(crate) fn set_col(&mut self, col: usize) {
        self.cursor.col = col.min(self.screen.cols() - 1);
        self.cursor.pending_wrap = false;
    }

    /// Относительный сдвиг по вертикали, не выходящий за область прокрутки.
    pub(crate) fn move_vertical(&mut self, delta: isize) {
        let row = self.cursor.row;
        let top = if row >= self.screen.top {
            self.screen.top
        } else {
            0
        };
        let last = self.screen.rows.len() - 1;
        let bottom = if row <= self.screen.bottom {
            self.screen.bottom
        } else {
            last
        };
        self.cursor.row = row.saturating_add_signed(delta).clamp(top, bottom);
        self.cursor.pending_wrap = false;
    }

    pub(crate) fn tab(&mut self, count: usize, forward: bool) {
        let mut col = self.cursor.col;
        for _ in 0..count.min(self.tabs.len()) {
            col = if forward {
                let last = self.tabs.len() - 1;
                (col + 1..last).find(|&c| self.tabs[c]).unwrap_or(last)
            } else {
                (0..col).rev().find(|&c| self.tabs[c]).unwrap_or(0)
            };
        }
        self.set_col(col);
    }

    // Затирая половину широкого символа, нужно убрать и вторую
    pub(crate) fn split_wide(&mut self, row: usize, start: usize, end: usize) {
        let cells = &mut self.screen.rows[row].cells;
        for edge in [start, end] {
            if edge > 0 && edge < cells.len() && cells[edge].style.flags.contains(Flags::WIDE_TAIL)
            {
                cells[edge - 1] = Cell::blank(cells[edge - 1].style);
                cells[edge] = Cell::blank(cells[edge].style);
            }
        }
    }

    pub(crate) fn erase_cells(&mut self, start: usize, end: usize) {
        let row = self.cursor.row;
        let end = end.min(self.screen.cols());
        self.split_wide(row, start, end);
        let fill = self.blank();
        self.screen.rows[row].cells[start..end].fill(fill);
        self.screen.rows[row]
            .images
            .retain(|p| !(start..end).contains(&p.col));
        if end == self.screen.cols() {
            self.screen.rows[row].wrapped = false;
        }
        self.cursor.pending_wrap = false;
    }

    fn erase_rows(&mut self, range: std::ops::Range<usize>) {
        let blank = Row::new(self.screen.cols(), &self.blank());
        self.screen.rows[range].fill(blank);
    }

    /// EL: 0 — до конца строки, 1 — от начала, 2 — всю.
    pub(crate) fn erase_line(&mut self, mode: u16) {
        let (col, cols) = (self.cursor.col, self.screen.cols());
        match mode {
            0 => self.erase_cells(col, cols),
            1 => self.erase_cells(0, col + 1),
            2 => self.erase_cells(0, cols),
            _ => {}
        }
    }

    /// ED: как EL для всего экрана, 3 — очистка истории.
    pub(crate) fn erase_display(&mut self, mode: u16) {
        let (row, count) = (self.cursor.row, self.screen.rows.len());
        match mode {
            0 => {
                self.erase_line(0);
                self.erase_rows(row + 1..count);
            }
            1 => {
                self.erase_line(1);
                self.erase_rows(0..row);
            }
            2 => {
                self.erase_rows(0..count);
                self.remove_overhanging_images();
            }
            3 => self.screen.history.clear(),
            _ => {}
        }
    }

    pub(crate) fn insert_blank(&mut self, count: usize) {
        let (row, col) = (self.cursor.row, self.cursor.col);
        let count = count.min(self.screen.cols() - col);
        let fill = self.blank();
        self.split_wide(row, col, self.screen.cols() - count);
        let cells = &mut self.screen.rows[row].cells;
        cells[col..].rotate_right(count);
        cells[col..col + count].fill(fill);
    }

    pub(crate) fn delete_chars(&mut self, count: usize) {
        let (row, col) = (self.cursor.row, self.cursor.col);
        let count = count.min(self.screen.cols() - col);
        let fill = self.blank();
        self.split_wide(row, col, col + count);
        let cells = &mut self.screen.rows[row].cells;
        cells[col..].rotate_left(count);
        let tail = cells.len() - count;
        cells[tail..].fill(fill);
        self.cursor.pending_wrap = false;
    }

    /// IL/DL: вставка и удаление строк действуют только внутри области прокрутки.
    pub(crate) fn shift_lines(&mut self, count: usize, insert: bool) {
        if !(self.screen.top..=self.screen.bottom).contains(&self.cursor.row) {
            return;
        }
        let top = std::mem::replace(&mut self.screen.top, self.cursor.row);
        let fill = self.blank();
        if insert {
            self.screen.scroll_down(count, &fill);
        } else {
            self.delete_lines(count, &fill);
        }
        self.screen.top = top;
        self.set_col(0);
    }

    // удалённые строки не должны попадать в историю, даже если top == 0
    fn delete_lines(&mut self, count: usize, fill: &Cell) {
        let (top, bottom) = (self.screen.top, self.screen.bottom);
        let count = count.min(bottom - top + 1);
        self.screen.rows[top..=bottom].rotate_left(count);
        let blank = Row::new(self.screen.cols(), fill);
        self.screen.rows[bottom + 1 - count..=bottom].fill(blank);
    }

    pub(crate) fn set_scroll_region(&mut self, top: usize, bottom: usize) {
        let bottom = bottom.min(self.screen.rows.len() - 1);
        if top < bottom {
            self.screen.top = top;
            self.screen.bottom = bottom;
            self.goto(0, 0);
        }
    }

    pub(crate) fn save_cursor(&mut self) {
        self.saved[usize::from(self.alternate)] = self.cursor;
    }

    pub(crate) fn restore_cursor(&mut self) {
        self.cursor = self.saved[usize::from(self.alternate)];
        self.cursor.row = self.cursor.row.min(self.screen.rows.len() - 1);
        self.cursor.col = self.cursor.col.min(self.screen.cols() - 1);
    }

    /// Переключает основной и альтернативный экраны (nvim, htop, less).
    pub(crate) fn set_alternate(&mut self, on: bool) {
        if self.alternate == on {
            return;
        }
        std::mem::swap(&mut self.screen, &mut self.other);
        self.alternate = on;
        if on {
            self.entered_full_screen();
            let count = self.screen.rows.len();
            self.erase_rows(0..count);
        }
    }
}
