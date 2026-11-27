//! Эмулятор: объединяет парсер, оба экрана, курсор и режимы.

use dimaster_vt::Parser;
use unicode_width::UnicodeWidthChar;

use crate::colors::ColorOverrides;
use crate::command::Commands;
use crate::extensions::{Policy, Progress};
use crate::graphics::{DecodedImage, SlowImage, Store, VirtualPlacement};
use crate::hyperlink::Links;
use crate::keyboard::KeyboardStack;
use crate::reflow::Position;
use crate::{Cell, Cursor, CursorShape, DefaultColors, Flags, Modes, PromptMark, Screen};
use crate::{Finished, SmartOptions};

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
    pub(crate) overrides: ColorOverrides,
    pub(crate) commands: Commands,
    pub(crate) links: Links,
    // стек заголовков окна (CSI 22/23 t)
    pub(crate) titles: Vec<String>,
    // запросы записи в буфер обмена: (primary, текст)
    pub(crate) clipboard: Vec<(bool, String)>,
    // форма, к которой возвращает DECSCUSR 0
    pub(crate) default_cursor: CursorShape,
    pub(crate) graphics: Store,
    // картинка, на которой разбор остановился
    pub(crate) deferred: Option<SlowImage>,
    pub(crate) notifications: Vec<(String, String)>,
    // сколько раз прозвенел BEL
    pub(crate) bells: u64,
    pub(crate) pointer: Option<String>,
    pub(crate) progress: Option<Progress>,
    pub(crate) policy: Policy,
}

impl Terminal {
    pub fn new(size: Size) -> Self {
        Self {
            parser: Parser::default(),
            state: State::new(size.clamped()),
        }
    }

    /// Разбирает вывод целиком, декодируя картинки на месте.
    pub fn advance(&mut self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            let (used, slow) = self.advance_part(bytes);
            bytes = &bytes[used..];
            if let Some(slow) = slow {
                self.finish_image(slow.decode());
            }
        }
    }

    /// Разбирает до первой картинки для внешнего декодера. Возвращает число
    /// разобранных байтов и эту картинку; остаток передаётся после
    /// [`Terminal::finish_image`].
    pub fn advance_part(&mut self, bytes: &[u8]) -> (usize, Option<SlowImage>) {
        let used = self.parser.advance(&mut self.state, bytes);
        (used, self.state.deferred.take())
    }

    pub fn finish_image(&mut self, image: DecodedImage) {
        self.state.finish_image(image);
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

    /// Виртуальное размещение Kitty (`U=1`) для Unicode-плейсхолдеров.
    pub fn virtual_placement(&self, image_id: u32, placement_id: u32) -> Option<VirtualPlacement> {
        self.state
            .graphics
            .get_virtual(image_id, placement_id)
            .cloned()
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

    /// Счётчик звонков: окно замечает новые по его росту.
    pub fn bells(&self) -> u64 {
        self.state.bells
    }

    /// Форма указателя, которую просит программа (OSC 22).
    pub fn pointer_shape(&self) -> Option<&str> {
        self.state.pointer.as_deref()
    }

    pub fn progress(&self) -> Option<Progress> {
        self.state.progress
    }

    pub fn set_policy(&mut self, policy: Policy) {
        self.state.policy = policy;
        if !policy.colors {
            self.state.overrides = ColorOverrides::default();
        }
    }

    pub fn color_overrides(&self) -> &ColorOverrides {
        &self.state.overrides
    }

    /// Тексты, которые программы просят положить в буфер обмена (OSC 52).
    pub fn take_clipboard_writes(&mut self) -> Vec<(bool, String)> {
        std::mem::take(&mut self.state.clipboard)
    }

    /// Ближайшая строка приглашения (OSC 133) до или после `line`.
    pub fn prompt_line(&self, line: u64, forward: bool) -> Option<u64> {
        let range = self.line_range();
        let is_prompt = |at: &u64| {
            let mark = self.line(*at).and_then(|r| r.mark);
            matches!(mark, Some(PromptMark::Prompt | PromptMark::Failed))
        };
        match forward {
            true => (line + 1..range.end).find(is_prompt),
            false => (range.start..line).rev().find(is_prompt),
        }
    }

    pub fn set_smart_options(&mut self, options: SmartOptions) {
        self.state.commands.options = options;
    }

    /// Завершившиеся команды, от старых к новым.
    pub fn commands(&self) -> impl DoubleEndedIterator<Item = &Finished> {
        self.state.commands.history.iter()
    }

    pub fn completed_commands(&self) -> u64 {
        self.state.commands.completed
    }

    /// Позиция `(строка, колонка)` начала ввода текущей команды (из `OSC 133;B`).
    pub fn input_start(&self) -> Option<(u64, usize)> {
        if self.state.alternate || self.state.commands.is_running() {
            return None;
        }
        self.state.commands.input_start
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
    // плоское перечисление начальных значений без логики
    #[allow(clippy::too_many_lines)]
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
            overrides: ColorOverrides::default(),
            commands: Commands::default(),
            links: Links::default(),
            titles: Vec::new(),
            clipboard: Vec::new(),
            default_cursor: CursorShape::Block,
            graphics: Store::default(),
            deferred: None,
            notifications: Vec::new(),
            bells: 0,
            pointer: None,
            progress: None,
            policy: Policy::default(),
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
        self.screen.reset_margins();
        self.other.reset_margins();
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
            let room = self.right_edge() + 1 - self.cursor.col;
            let (chunk, rest) = run.split_at(run.len().min(room));
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
        let overflow = self.cursor.col + width > self.right_edge() + 1;
        if (self.cursor.pending_wrap || overflow) && self.modes.autowrap {
            // перенос внутри полей не склеивает строки при смене ширины
            self.screen.rows[self.cursor.row].wrapped = !self.screen.narrowed();
            self.cursor.col = self.left_edge();
            self.line_feed();
        }
        let last = self.right_edge() + 1;
        self.cursor.col = self.cursor.col.min(last.saturating_sub(width));
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
        let last = self.right_edge();
        if next <= last {
            self.cursor.col = next;
        } else {
            self.cursor.col = last;
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
