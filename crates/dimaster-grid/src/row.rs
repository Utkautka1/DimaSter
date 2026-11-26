//! Строка сетки вместе с метаданными переноса и shell-интеграции.

use crate::{Cell, Flags};

/// Метки OSC 133: по ним будет работать переход между командами.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptMark {
    Prompt,
    Output,
}

/// Строка экрана или истории (в истории хвостовые пустые клетки обрезаны).
#[derive(Debug, Clone)]
pub struct Row {
    pub cells: Vec<Cell>,
    /// Строка продолжается на следующей (мягкий перенос).
    pub wrapped: bool,
    pub mark: Option<PromptMark>,
}

impl Row {
    pub fn new(cols: usize, fill: &Cell) -> Self {
        Self {
            cells: vec![fill.clone(); cols],
            wrapped: false,
            mark: None,
        }
    }

    // заглушка без выделения памяти на время обмена буферами
    pub(crate) fn empty() -> Self {
        Self {
            cells: Vec::new(),
            wrapped: false,
            mark: None,
        }
    }

    pub(crate) fn reset(&mut self, cols: usize, fill: &Cell) {
        self.cells.clear();
        self.cells.resize(cols, fill.clone());
        self.wrapped = false;
        self.mark = None;
    }

    /// Текст строки без хвостовых пробелов и вторых половин широких символов.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for cell in self
            .cells
            .iter()
            .filter(|c| !c.style.flags.contains(Flags::WIDE_TAIL))
        {
            out.push(cell.ch);
            if let Some(extra) = &cell.extra {
                out.push_str(extra);
            }
        }
        out.truncate(out.trim_end().len());
        out
    }

    pub(crate) fn resize(&mut self, cols: usize) {
        self.cells.resize(cols, Cell::default());
    }
}
