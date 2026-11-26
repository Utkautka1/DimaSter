//! Эмулятор: объединяет парсер, оба экрана, курсор и режимы.

use dimaster_vt::Parser;
use unicode_width::UnicodeWidthChar;

use crate::graphics::Store;
use crate::keyboard::KeyboardStack;
use crate::reflow::Position;
use crate::{Cell, Cursor, CursorShape, DefaultColors, Flags, Modes, Screen};

/// Размер сетки в ячейках.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub rows: usize,
    pub cols: usize,
}

impl Size {
    // нулевой размер сломал бы всю арифметику курсора
    fn clamped(self) -> Self {
        Self {
            rows: self.rows.max(1),
            cols: self.cols.max(2),
        }
    }
}

/// Строки истории по умолчанию.
const HISTORY_LIMIT: usize = 10_000;

/// Терминал: байты программы на входе, состояние экрана на выходе.
#[derive(Debug)]
pub struct Terminal {
    parser: Parser,
    pub(crate) state: State,
}

/// Всё, что меняют команды парсера (отдельно от него из-за заимствований).
#[derive(Debug)]
pub(crate) struct State {
    pub(crate) screen: Screen,
    // неактивный экран: альтернативный или спрятанный основной
    pub(crate) other: Screen,
    pub(crate) alternate: bool,
    pub(crate) cursor: Cursor,
    // DECSC хранится отдельно для основного [0] и альтернативного [1] экранов
    pub(crate) saved: [Cursor; 2],
    pub(crate) modes: Modes,
    pub(crate) tabs: Vec<bool>,
    pub(crate) title: String,
    pub(crate) cwd: Option<String>,
    pub(crate) reply: Vec<u8>,
    // последний напечатанный символ для REP (CSI b)
    pub(crate) last_char: Option<char>,
    pub(crate) keyboard: KeyboardStack,
    // ширина и высота ячейки в пикселях, задаёт окно
    pub(crate) cell_pixels: (usize, usize),
    pub(crate) colors: DefaultColors,
    // форма, к которой возвращает DECSCUSR 0
    pub(crate) default_cursor: CursorShape,
    pub(crate) graphics: Store,
    pub(crate) notifications: Vec<(String, String)>,
}

impl Terminal {
    pub fn new(size: Size) -> Self {
        Self {
            parser: Parser::default(),
            state: State::new(size.clamped()),
        }
    }

    pub fn advance(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.state, bytes);
    }

    pub fn resize(&mut self, size: Size) {
        self.state.resize(size.clamped());
    }

    pub fn screen(&self) -> &Screen {
        &self.state.screen
    }

    pub fn cursor(&self) -> Cursor {
        self.state.cursor
    }

    pub fn modes(&self) -> Modes {
        self.state.modes
    }

    pub fn is_alternate(&self) -> bool {
        self.state.alternate
    }

    /// Флаги клавиатурного протокола Kitty для активного экрана.
    pub fn keyboard_flags(&self) -> u16 {
        self.state.keyboard.current(self.state.alternate)
    }

    pub fn set_cell_pixels(&mut self, width: usize, height: usize) {
        self.state.cell_pixels = (width, height);
    }

    /// Цвета темы, которые терминал сообщает программам.
    pub fn set_default_colors(&mut self, colors: DefaultColors) {
        self.state.colors = colors;
    }

    pub fn set_default_cursor(&mut self, shape: CursorShape) {
        self.state.default_cursor = shape;
        self.state.modes.cursor_shape = shape;
    }

    /// Ограничивает историю основного экрана; лишние старые строки удаляются.
    pub fn set_history_limit(&mut self, lines: usize) {
        let state = &mut self.state;
        let main = if state.alternate {
            &mut state.other
        } else {
            &mut state.screen
        };
        main.set_limit(lines);
    }

    /// `false` — картинки исчезают, когда их строка уходит в историю.
    pub fn set_images_scroll(&mut self, keep: bool) {
        let state = &mut self.state;
        let main = if state.alternate {
            &mut state.other
        } else {
            &mut state.screen
        };
        main.keep_images = keep;
    }

    /// Уведомления, запрошенные программами (OSC 9 и 777): заголовок и текст.
    pub fn take_notifications(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.state.notifications)
    }

    pub fn title(&self) -> &str {
        &self.state.title
    }

    /// Рабочий каталог shell из OSC 7 (нужен для восстановления сессий).
    pub fn cwd(&self) -> Option<&str> {
        self.state.cwd.as_deref()
    }

    /// Забирает ответы, которые нужно записать в PTY.
    pub fn take_reply(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.state.reply)
    }

    /// Видимый текст построчно — для тестов и headless-режима.
    pub fn text(&self) -> String {
        let lines: Vec<_> = self
            .state
            .screen
            .rows()
            .iter()
            .map(crate::Row::text)
            .collect();
        lines.join("\n").trim_end().to_owned()
    }
}

// col == cols означает «следующий символ перенесётся»
fn place(cursor: &mut Cursor, position: Position, size: Size) {
    cursor.row = position.row;
    cursor.col = position.col.min(size.cols - 1);
    cursor.pending_wrap = position.col >= size.cols;
}

fn default_tabs(cols: usize) -> Vec<bool> {
    (0..cols).map(|c| c % 8 == 0).collect()
}

impl State {
    pub(crate) fn new(size: Size) -> Self {
        Self {
            screen: Screen::new(size, HISTORY_LIMIT),
            other: Screen::new(size, 0),
            alternate: false,
            cursor: Cursor::default(),
            saved: [Cursor::default(); 2],
            modes: Modes::default(),
            tabs: default_tabs(size.cols),
            title: String::new(),
            cwd: None,
            reply: Vec::new(),
            last_char: None,
            keyboard: KeyboardStack::default(),
            cell_pixels: (0, 0),
            colors: DefaultColors::default(),
            default_cursor: CursorShape::Block,
            graphics: Store::default(),
            notifications: Vec::new(),
        }
    }

    // переразбивается только основной экран, даже если он сейчас спрятан
    fn resize(&mut self, size: Size) {
        self.tabs = default_tabs(size.cols);
        if self.alternate {
            self.other_resize(size);
        } else {
            let col = self.cursor.col + usize::from(self.cursor.pending_wrap);
            let position = self.screen.reflow(
                size,
                Position {
                    row: self.cursor.row,
                    col,
                },
            );
            self.other.resize(size);
            place(&mut self.cursor, position, size);
        }
        self.cursor.row = self.cursor.row.min(size.rows - 1);
        self.cursor.col = self.cursor.col.min(size.cols - 1);
    }

    fn other_resize(&mut self, size: Size) {
        let saved = self.saved[0];
        let col = saved.col + usize::from(saved.pending_wrap);
        let position = self.other.reflow(
            size,
            Position {
                row: saved.row,
                col,
            },
        );
        place(&mut self.saved[0], position, size);
        self.screen.resize(size);
        self.cursor.pending_wrap = false;
    }

    /// Печать символа с учётом ширины, переноса и режима вставки.
    pub(crate) fn put(&mut self, ch: char) {
        let ch = self.cursor.charsets.map(ch);
        // ASCII — основной поток, таблицы Unicode для него не нужны
        let width = if ch.is_ascii() {
            1
        } else {
            ch.width().unwrap_or(0).min(2)
        };
        if width == 0 {
            return self.combine(ch);
        }
        self.make_room(width);
        if self.modes.insert {
            self.insert_blank(width);
        }
        self.write_cell(ch, width);
        self.move_after_print(width);
        self.last_char = Some(ch);
    }

    /// Пишет серию ASCII сразу в строку. Редкие режимы (вставка, псевдографика,
    /// без автопереноса) идут общим посимвольным путём.
    pub(crate) fn put_ascii(&mut self, mut run: &[u8]) {
        let charsets = self.cursor.charsets;
        if self.modes.insert || !self.modes.autowrap || charsets.graphics[charsets.active] {
            return run.iter().for_each(|&b| self.put(char::from(b)));
        }
        while !run.is_empty() {
            self.make_room(1);
            let (chunk, rest) = run.split_at(run.len().min(self.screen.cols() - self.cursor.col));
            self.write_ascii(chunk);
            run = rest;
        }
    }

    // `chunk` помещается в строку курсора
    fn write_ascii(&mut self, chunk: &[u8]) {
        let (row, col) = (self.cursor.row, self.cursor.col);
        let end = col + chunk.len();
        self.split_wide(row, col, end);
        if end == self.screen.cols() {
            self.screen.rows[row].wrapped = false;
        }
        let style = self.cursor.style;
        for (cell, &byte) in self.screen.rows[row].cells[col..end].iter_mut().zip(chunk) {
            cell.ch = char::from(byte);
            cell.extra = None;
            cell.style = style;
        }
        self.last_char = chunk.last().map(|&b| char::from(b));
        self.move_after_print(chunk.len());
    }

    fn make_room(&mut self, width: usize) {
        let overflow = self.cursor.col + width > self.screen.cols();
        if (self.cursor.pending_wrap || overflow) && self.modes.autowrap {
            self.screen.rows[self.cursor.row].wrapped = true;
            self.cursor.col = 0;
            self.line_feed();
        }
        self.cursor.col = self.cursor.col.min(self.screen.cols() - width);
        self.cursor.pending_wrap = false;
    }

    fn write_cell(&mut self, ch: char, width: usize) {
        let (row, col) = (self.cursor.row, self.cursor.col);
        self.split_wide(row, col, col + width);
        if col + width == self.screen.cols() {
            self.screen.rows[row].wrapped = false;
        }
        let mut style = self.cursor.style;
        style.flags.set(Flags::WIDE, width == 2);
        let cells = &mut self.screen.rows[row].cells;
        cells[col] = Cell::new(ch, style);
        if width == 2 {
            style.flags.set(Flags::WIDE, false);
            style.flags.set(Flags::WIDE_TAIL, true);
            cells[col + 1] = Cell::new(' ', style);
        }
    }

    fn move_after_print(&mut self, width: usize) {
        let next = self.cursor.col + width;
        if next < self.screen.cols() {
            self.cursor.col = next;
        } else {
            self.cursor.col = self.screen.cols() - 1;
            self.cursor.pending_wrap = self.modes.autowrap;
        }
    }

    // Знак нулевой ширины приклеивается к предыдущему символу
    fn combine(&mut self, mark: char) {
        let back = usize::from(!self.cursor.pending_wrap);
        let Some(mut col) = self.cursor.col.checked_sub(back) else {
            return;
        };
        let cells = &mut self.screen.rows[self.cursor.row].cells;
        if cells[col].style.flags.contains(Flags::WIDE_TAIL) {
            col = col.saturating_sub(1);
        }
        let extra = cells[col].extra.get_or_insert_default();
        // защита от «zalgo»-потока, раздувающего одну ячейку
        if extra.len() < 32 {
            extra.push(mark);
        }
    }
}
