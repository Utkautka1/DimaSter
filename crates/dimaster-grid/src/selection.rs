//! Выделение текста.
//!
//! Строки адресуются постоянными номерами (с учётом вытесненных из истории),
//! поэтому выделение не «уезжает», когда программа продолжает печатать.

use crate::{Flags, Row, Terminal};

/// Клетка в постоянных координатах.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    pub line: u64,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionKind {
    /// По символам.
    Simple,
    Word,
    Line,
    /// Прямоугольный блок.
    Block,
}

/// Выделение от точки нажатия до текущей.
#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub kind: SelectionKind,
    pub anchor: Point,
    pub head: Point,
}

/// Выделение, приведённое к упорядоченным границам (обе включительно).
#[derive(Debug, Clone, Copy)]
pub struct Span {
    start: Point,
    end: Point,
    block: bool,
}

impl Span {
    pub fn contains(&self, line: u64, col: usize) -> bool {
        if self.block {
            let (left, right) = (self.start.col, self.end.col);
            return (self.start.line..=self.end.line).contains(&line)
                && (left..=right).contains(&col);
        }
        (self.start..=self.end).contains(&Point { line, col })
    }

    // диапазон колонок, выделенных в строке
    fn columns(&self, line: u64) -> (usize, usize) {
        let first = if self.block || line == self.start.line {
            self.start.col
        } else {
            0
        };
        let last = if self.block || line == self.end.line {
            self.end.col
        } else {
            usize::MAX
        };
        (first, last)
    }
}

// символы, из которых состоят слова, пути и адреса
fn is_word(ch: char) -> bool {
    ch.is_alphanumeric() || "_-./~:@%+#?=&".contains(ch)
}

impl Terminal {
    /// Постоянный номер верхней видимой строки при прокрутке на `scroll`.
    pub fn visible_line(&self, scroll: usize) -> u64 {
        let screen = self.screen();
        screen.evicted + screen.history.len().saturating_sub(scroll) as u64
    }

    pub fn line(&self, line: u64) -> Option<&Row> {
        let screen = self.screen();
        let index = usize::try_from(line.checked_sub(screen.evicted)?).ok()?;
        match screen.history.get(index) {
            Some(row) => Some(row),
            None => screen.rows.get(index - screen.history.len()),
        }
    }

    /// Расширяет выделение до слов или строк и упорядочивает концы.
    pub fn span(&self, selection: Selection) -> Span {
        let Selection { kind, anchor, head } = selection;
        let (mut start, mut end) = (anchor.min(head), anchor.max(head));
        match kind {
            SelectionKind::Simple => {}
            SelectionKind::Word => {
                start.col = self.word_edge(start, false);
                end.col = self.word_edge(end, true);
            }
            SelectionKind::Line => (start.col, end.col) = (0, usize::MAX),
            SelectionKind::Block => {
                (start.col, end.col) = (anchor.col.min(head.col), anchor.col.max(head.col));
            }
        }
        Span {
            start,
            end,
            block: kind == SelectionKind::Block,
        }
    }

    fn word_edge(&self, from: Point, forward: bool) -> usize {
        let Some(row) = self.line(from.line) else {
            return from.col;
        };
        let word = |col: usize| {
            row.cells
                .get(col)
                .is_some_and(|c| c.style.flags.contains(Flags::WIDE_TAIL) || is_word(c.ch))
        };
        let mut col = from.col;
        if !word(col) {
            return col;
        }
        while let Some(next) = if forward {
            col.checked_add(1)
        } else {
            col.checked_sub(1)
        } && word(next)
        {
            col = next;
        }
        col
    }

    /// Текст выделения; мягкие переносы не превращаются в переводы строк.
    pub fn selected_text(&self, selection: Selection) -> String {
        let span = self.span(selection);
        let mut out = String::new();
        for line in span.start.line..=span.end.line {
            let Some(row) = self.line(line) else { continue };
            let (first, last) = span.columns(line);
            let text = row.slice_text(first, last);
            let joined = row.wrapped && !span.block;
            out.push_str(if joined { &text } else { text.trim_end() });
            if !joined && line != span.end.line {
                out.push('\n');
            }
        }
        out
    }
}
