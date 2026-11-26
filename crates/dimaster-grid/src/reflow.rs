//! Переразбиение строк основного экрана при изменении размера.
//!
//! Строки, соединённые мягким переносом, склеиваются в логическую строку и
//! режутся заново под новую ширину. Позиция курсора переносится вместе с
//! символом, на котором он стоял.

use std::collections::VecDeque;

use crate::{Cell, Flags, Placement, PromptMark, Row, Screen, Size};

/// Положение курсора; `col` может равняться ширине при отложенном переносе.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Position {
    pub(crate) row: usize,
    pub(crate) col: usize,
}

#[derive(Default)]
struct Line {
    cells: Vec<Cell>,
    mark: Option<PromptMark>,
    // картинки со смещением внутри логической строки
    images: Vec<(usize, Placement)>,
    // смещение курсора внутри логической строки
    cursor: Option<usize>,
}

impl Screen {
    /// Меняет размер с переразбиением и возвращает новую позицию курсора.
    pub(crate) fn reflow(&mut self, size: Size, cursor: Position) -> Position {
        let cursor = Position {
            row: cursor.row.min(self.rows.len() - 1),
            ..cursor
        };
        let absolute = Position {
            row: self.history.len() + cursor.row,
            ..cursor
        };
        let mut all: Vec<Row> = std::mem::take(&mut self.history)
            .into_iter()
            .map(|r| r.unpack())
            .collect();
        all.append(&mut self.rows);
        // пустые строки под курсором не должны выталкивать текст в историю
        while all.len() > absolute.row + 1 && all.last().is_some_and(Row::is_blank) {
            all.pop();
        }
        let (rows, position) = wrap(join(all, absolute), size.cols);
        self.distribute(rows, position, size)
    }

    // делит строки между историей и экраном так, чтобы курсор остался на экране
    fn distribute(&mut self, mut rows: Vec<Row>, cursor: Position, size: Size) -> Position {
        let split = rows.len().saturating_sub(size.rows).min(cursor.row);
        let visible = rows.split_off(split);
        self.history = VecDeque::new();
        for mut row in rows {
            self.remember(&mut row);
        }
        self.rows = visible;
        self.rows
            .resize(size.rows, Row::new(size.cols, &Cell::default()));
        self.top = 0;
        self.bottom = size.rows - 1;
        Position {
            row: cursor.row - split,
            col: cursor.col,
        }
    }
}

fn join(rows: Vec<Row>, cursor: Position) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut line = Line::default();
    let mut rows = rows.into_iter().enumerate().peekable();
    while let Some((index, mut row)) = rows.next() {
        if index == cursor.row {
            line.cursor = Some(line.cells.len() + cursor.col);
        }
        line.mark = line.mark.or(row.mark);
        let offset = line.cells.len();
        line.images
            .extend(row.images.drain(..).map(|p| (offset + p.col, p)));
        if row.wrapped && rows.peek().is_some_and(|(_, next)| next.starts_wide()) {
            drop_spacer(&mut row.cells);
        }
        line.cells.append(&mut row.cells);
        if !row.wrapped {
            lines.push(std::mem::take(&mut line));
        }
    }
    if !line.cells.is_empty() || line.cursor.is_some() || !line.images.is_empty() {
        lines.push(line);
    }
    lines
}

// пустая клетка, оставленная потому, что широкий символ не влез в конец строки
fn drop_spacer(cells: &mut Vec<Cell>) {
    if cells.last() == Some(&Cell::default()) {
        cells.pop();
    }
}

fn wrap(lines: Vec<Line>, cols: usize) -> (Vec<Row>, Position) {
    let mut rows = Vec::new();
    let mut cursor = Position { row: 0, col: 0 };
    for line in lines {
        wrap_line(line, cols, &mut rows, &mut cursor);
    }
    (rows, cursor)
}

fn wrap_line(mut line: Line, cols: usize, rows: &mut Vec<Row>, cursor: &mut Position) {
    trim(&mut line.cells);
    let mut start = 0;
    loop {
        let end = chunk_end(&line.cells, start, cols);
        let last = end >= line.cells.len();
        track_cursor(line.cursor, start..end, last, rows.len(), cols, cursor);
        let mark = line.mark.filter(|_| start == 0);
        let mut row = chunk_row(&line.cells[start..end], cols, !last, mark);
        row.images = chunk_images(
            &line.images,
            start,
            if last { usize::MAX } else { end },
            cols,
        );
        rows.push(row);
        if last {
            return;
        }
        start = end;
    }
}

// курсор попадает в тот кусок, где лежит его смещение
fn track_cursor(
    offset: Option<usize>,
    chunk: std::ops::Range<usize>,
    last: bool,
    row: usize,
    cols: usize,
    cursor: &mut Position,
) {
    if let Some(offset) = offset
        && offset >= chunk.start
        && (offset < chunk.end || last)
    {
        *cursor = Position {
            row,
            col: (offset - chunk.start).min(cols),
        };
    }
}

fn trim(cells: &mut Vec<Cell>) {
    let blank = Cell::default();
    let used = cells.iter().rposition(|c| *c != blank).map_or(0, |i| i + 1);
    cells.truncate(used);
}

fn chunk_row(chunk: &[Cell], cols: usize, wrapped: bool, mark: Option<PromptMark>) -> Row {
    let mut cells = chunk.to_vec();
    cells.resize(cols, Cell::default());
    Row {
        cells,
        wrapped,
        mark,
        images: Vec::new(),
    }
}

fn chunk_images(
    images: &[(usize, Placement)],
    start: usize,
    end: usize,
    cols: usize,
) -> Vec<Placement> {
    images
        .iter()
        .filter(|(offset, _)| (start..end).contains(offset))
        .map(|(offset, p)| Placement {
            col: (offset - start).min(cols - 1),
            ..p.clone()
        })
        .collect()
}

// широкий символ нельзя разрывать между строками
fn chunk_end(cells: &[Cell], start: usize, cols: usize) -> usize {
    let end = (start + cols).min(cells.len());
    let splits = end < cells.len() && cells[end].style.flags.contains(Flags::WIDE_TAIL);
    if splits { end - 1 } else { end }
}
