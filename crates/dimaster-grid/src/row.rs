//! Строка сетки вместе с метаданными переноса и shell-интеграции.

use crate::{Cell, Flags, Placement};

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
    /// Картинки, начинающиеся на этой строке.
    pub images: Vec<Placement>,
}

impl Row {
    pub fn new(cols: usize, fill: &Cell) -> Self {
        Self {
            cells: vec![fill.clone(); cols],
            wrapped: false,
            mark: None,
            images: Vec::new(),
        }
    }

    // заглушка без выделения памяти на время обмена буферами
    pub(crate) fn empty() -> Self {
        Self {
            cells: Vec::new(),
            wrapped: false,
            mark: None,
            images: Vec::new(),
        }
    }

    // сбрасывает метаданные строки с уже пустыми клетками
    pub(crate) fn recycle(&mut self) {
        self.wrapped = false;
        self.mark = None;
        self.images.clear();
    }

    pub(crate) fn reset(&mut self, cols: usize, fill: &Cell) {
        self.cells.clear();
        self.cells.resize(cols, fill.clone());
        self.wrapped = false;
        self.mark = None;
        self.images.clear();
    }

    pub(crate) fn is_blank(&self) -> bool {
        !self.wrapped && self.images.is_empty() && self.cells.iter().all(|c| *c == Cell::default())
    }

    pub(crate) fn starts_wide(&self) -> bool {
        self.cells
            .first()
            .is_some_and(|c| c.style.flags.contains(Flags::WIDE))
    }

    /// Текст строки без хвостовых пробелов и вторых половин широких символов.
    pub fn text(&self) -> String {
        let mut out = self.slice_text(0, usize::MAX);
        out.truncate(out.trim_end().len());
        out
    }

    /// Текст колонок `first..=last` как есть.
    pub(crate) fn slice_text(&self, first: usize, last: usize) -> String {
        let count = last.saturating_sub(first).saturating_add(1);
        let mut out = String::new();
        let cells = self.cells.iter().skip(first).take(count);
        for cell in cells.filter(|c| !c.style.flags.contains(Flags::WIDE_TAIL)) {
            out.push(cell.ch);
            if let Some(extra) = &cell.extra {
                out.push_str(extra);
            }
        }
        out
    }

    pub(crate) fn resize(&mut self, cols: usize) {
        self.cells.resize(cols, Cell::default());
    }
}
