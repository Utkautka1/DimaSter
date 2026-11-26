//! Сжатое хранение строк истории (RLE и общие отрезки стиля).

use crate::{Cell, Flags, Placement, PromptMark, Row, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellRun {
    Repeat { count: u16, ch: char, style: Style },
    Text { chars: u16, style: Style },
}

/// Строка в истории: текст хранится плотно в UTF-8, стили — прогонами.
#[derive(Debug, Clone)]
pub(crate) struct PackedRow {
    pub(crate) wrapped: bool,
    pub(crate) mark: Option<PromptMark>,
    pub(crate) images: Vec<Placement>,
    text: Box<str>,
    runs: Box<[CellRun]>,
    extras: Box<[(u16, Box<str>)]>,
}

impl PackedRow {
    /// Сжимает ячейки из `cells` и метаданные строки.
    pub(crate) fn pack(
        cells: &mut [Cell],
        wrapped: bool,
        mark: Option<PromptMark>,
        images: Vec<Placement>,
    ) -> Self {
        let used = used_len(cells, wrapped);
        let (mut text, mut runs, mut extras) = (String::new(), Vec::new(), Vec::new());
        pack_cells(&mut cells[..used], &mut text, &mut runs, &mut extras);
        Self {
            wrapped,
            mark,
            images,
            text: text.into_boxed_str(),
            runs: runs.into_boxed_slice(),
            extras: extras.into_boxed_slice(),
        }
    }

    /// Восстанавливает полную строку для отрисовки, выделения или reflow.
    pub(crate) fn unpack(&self) -> Row {
        let count = self.cell_count();
        let mut cells = Vec::with_capacity(count);
        let mut chars = self.text.chars();
        for &run in &self.runs {
            unpack_run(run, &mut chars, &mut cells);
        }
        for (col, extra) in &self.extras {
            if let Some(cell) = cells.get_mut(usize::from(*col)) {
                cell.extra = Some(Box::new(extra.as_ref().to_owned()));
            }
        }
        Row {
            cells,
            wrapped: self.wrapped,
            mark: self.mark,
            images: self.images.clone(),
        }
    }

    /// Текст строки без распаковки всех ячеек.
    pub(crate) fn text(&self) -> String {
        let mut out = String::with_capacity(self.text.len());
        let mut chars = self.text.chars();
        let mut col = 0usize;
        for &run in &self.runs {
            append_run_text(run, &mut chars, &self.extras, &mut col, &mut out);
        }
        out.truncate(out.trim_end().len());
        out
    }

    /// Оценочный размер строки в куче и структуре (в байтах).
    pub fn heap_bytes(&self) -> usize {
        size_of::<Self>()
            + self.text.len()
            + std::mem::size_of_val(&*self.runs)
            + self.extras.iter().map(|(_, s)| 18 + s.len()).sum::<usize>()
            + self.images.capacity() * size_of::<Placement>()
    }

    fn cell_count(&self) -> usize {
        self.runs
            .iter()
            .map(|r| match *r {
                CellRun::Repeat { count, .. } => usize::from(count),
                CellRun::Text { chars, .. } => usize::from(chars),
            })
            .sum()
    }
}

fn used_len(cells: &[Cell], wrapped: bool) -> usize {
    if wrapped {
        return cells.len();
    }
    let blank = Cell::default();
    cells.iter().rposition(|c| *c != blank).map_or(0, |i| i + 1)
}

fn pack_cells(
    cells: &mut [Cell],
    text: &mut String,
    runs: &mut Vec<CellRun>,
    extras: &mut Vec<(u16, Box<str>)>,
) {
    let mut i = 0;
    while i < cells.len() {
        if let Some(count) = repeat_len(&cells[i..]) {
            runs.push(CellRun::Repeat {
                count: count as u16,
                ch: cells[i].ch,
                style: cells[i].style,
            });
            i += count;
            continue;
        }
        i += pack_text_run(&mut cells[i..], i, text, runs, extras);
    }
}

fn repeat_len(slice: &[Cell]) -> Option<usize> {
    let first = &slice[0];
    if first.extra.is_some() {
        return None;
    }
    let count = slice
        .iter()
        .take(u16::MAX as usize)
        .take_while(|c| c.extra.is_none() && c.ch == first.ch && c.style == first.style)
        .count();
    (count >= 3).then_some(count)
}

fn pack_text_run(
    slice: &mut [Cell],
    base_col: usize,
    text: &mut String,
    runs: &mut Vec<CellRun>,
    extras: &mut Vec<(u16, Box<str>)>,
) -> usize {
    let style = slice[0].style;
    let limit = slice.len().min(u16::MAX as usize);
    let mut count = 0;
    while count < limit {
        if slice[count].style != style || (count > 0 && repeat_len(&slice[count..]).is_some()) {
            break;
        }
        text.push(slice[count].ch);
        if let Some(extra) = slice[count].extra.take() {
            extras.push(((base_col + count) as u16, extra.into_boxed_str()));
        }
        slice[count] = Cell::default();
        count += 1;
    }
    runs.push(CellRun::Text {
        chars: count as u16,
        style,
    });
    count
}

fn unpack_run(run: CellRun, chars: &mut std::str::Chars<'_>, out: &mut Vec<Cell>) {
    match run {
        CellRun::Repeat { count, ch, style } => {
            out.extend(std::iter::repeat_n(
                Cell::new(ch, style),
                usize::from(count),
            ));
        }
        CellRun::Text { chars: n, style } => {
            for ch in chars.take(usize::from(n)) {
                out.push(Cell::new(ch, style));
            }
        }
    }
}

fn append_run_text(
    run: CellRun,
    chars: &mut std::str::Chars<'_>,
    extras: &[(u16, Box<str>)],
    col: &mut usize,
    out: &mut String,
) {
    match run {
        CellRun::Repeat { count, ch, style } => {
            if !style.flags.contains(Flags::WIDE_TAIL) {
                out.extend(std::iter::repeat_n(ch, usize::from(count)));
            }
            *col += usize::from(count);
        }
        CellRun::Text { chars: n, style } => {
            let tail = style.flags.contains(Flags::WIDE_TAIL);
            for ch in chars.take(usize::from(n)) {
                if !tail {
                    out.push(ch);
                    if let Some((_, extra)) = extras.iter().find(|(c, _)| usize::from(*c) == *col) {
                        out.push_str(extra);
                    }
                }
                *col += 1;
            }
        }
    }
}
