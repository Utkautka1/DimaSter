//! Vi-режим: перемещение по выводу и копирование без мыши.

use dimaster_grid::{Point, Row, Selection, SelectionKind, Terminal};

use super::Press;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Vi {
    pub(crate) cursor: Point,
    // начало выделения и его вид
    anchor: Option<(Point, SelectionKind)>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Stay,
    Exit,
    /// Скопировать выделение и выйти.
    Yank,
    Search,
}

fn chars(term: &Terminal, line: u64) -> Vec<char> {
    term.line(line)
        .map(Row::text)
        .unwrap_or_default()
        .chars()
        .collect()
}

// начало следующего слова; None — дальше в этой строке слов нет
fn next_word(text: &[char], from: usize) -> Option<usize> {
    let gap = (from..text.len()).find(|&i| text[i].is_whitespace())?;
    (gap..text.len()).find(|&i| !text[i].is_whitespace())
}

fn previous_word(text: &[char], from: usize) -> Option<usize> {
    let end = (0..from.min(text.len())).rfind(|&i| !text[i].is_whitespace())?;
    Some(
        (0..end)
            .rfind(|&i| text[i].is_whitespace())
            .map_or(0, |i| i + 1),
    )
}

impl Vi {
    pub(crate) fn new(cursor: Point) -> Self {
        Self {
            cursor,
            anchor: None,
        }
    }

    pub(crate) fn selection(&self) -> Option<Selection> {
        let (anchor, kind) = self.anchor?;
        Some(Selection {
            kind,
            anchor,
            head: self.cursor,
        })
    }

    fn line_by(&mut self, delta: i64, term: &Terminal) {
        let range = term.line_range();
        let line = self.cursor.line.saturating_add_signed(delta);
        self.cursor.line = line.clamp(range.start, range.end - 1);
    }

    fn word(&mut self, forward: bool, term: &Terminal) {
        let text = chars(term, self.cursor.line);
        let found = match forward {
            true => next_word(&text, self.cursor.col),
            false => previous_word(&text, self.cursor.col),
        };
        match found {
            Some(col) => self.cursor.col = col,
            None => {
                self.line_by(if forward { 1 } else { -1 }, term);
                let text = chars(term, self.cursor.line);
                let edge = if forward { 0 } else { text.len() };
                self.cursor.col = previous_word(&text, edge).filter(|_| !forward).unwrap_or(0);
            }
        }
    }

    fn motion(&mut self, press: Press, term: &Terminal) {
        let page = term.screen().rows().len() as i64;
        let last_col = term.screen().rows()[0].cells.len() - 1;
        match press {
            Press::Char('h') | Press::Left => self.cursor.col = self.cursor.col.saturating_sub(1),
            Press::Char('l') | Press::Right => {
                self.cursor.col = (self.cursor.col + 1).min(last_col)
            }
            Press::Char('j') | Press::Down => self.line_by(1, term),
            Press::Char('k') | Press::Up => self.line_by(-1, term),
            Press::Char('0') => self.cursor.col = 0,
            Press::Char('$') => {
                self.cursor.col = chars(term, self.cursor.line).len().saturating_sub(1);
            }
            Press::Char('w') => self.word(true, term),
            Press::Char('b') => self.word(false, term),
            Press::Char('g') => self.line_by(i64::MIN, term),
            Press::Char('G') => self.line_by(i64::MAX, term),
            Press::Ctrl('u') => self.line_by(-page / 2, term),
            Press::Ctrl('d') => self.line_by(page / 2, term),
            Press::Ctrl('b') => self.line_by(-page, term),
            Press::Ctrl('f') => self.line_by(page, term),
            _ => {}
        }
    }

    // повторное нажатие той же клавиши снимает выделение
    fn toggle(&mut self, kind: SelectionKind) {
        self.anchor = match self.anchor {
            Some((_, current)) if current == kind => None,
            Some((anchor, _)) => Some((anchor, kind)),
            None => Some((self.cursor, kind)),
        };
    }

    pub(crate) fn press(&mut self, press: Press, term: &Terminal) -> Outcome {
        match press {
            Press::Escape if self.anchor.is_some() => self.anchor = None,
            Press::Escape | Press::Char('q' | 'i') => return Outcome::Exit,
            Press::Char('y') => return Outcome::Yank,
            Press::Char('/') => return Outcome::Search,
            Press::Char('v') => self.toggle(SelectionKind::Simple),
            Press::Char('V') => self.toggle(SelectionKind::Line),
            Press::Ctrl('v') => self.toggle(SelectionKind::Block),
            _ => self.motion(press, term),
        }
        Outcome::Stay
    }
}

#[cfg(test)]
mod tests {
    use dimaster_grid::Size;

    use super::*;

    #[test]
    fn motions_selection_and_yank() {
        let mut term = Terminal::new(Size { rows: 3, cols: 20 });
        term.advance(b"one two three\r\nfour");
        let mut vi = Vi::new(Point { line: 0, col: 0 });
        let keys =
            |vi: &mut Vi, text: &str| text.chars().map(|c| vi.press(Press::Char(c), &term)).last();
        keys(&mut vi, "ww");
        assert_eq!(vi.cursor.col, 8);
        keys(&mut vi, "b");
        assert_eq!(vi.cursor.col, 4);
        keys(&mut vi, "v$");
        assert_eq!(term.selected_text(vi.selection().unwrap()), "two three");
        assert_eq!(keys(&mut vi, "y"), Some(Outcome::Yank));
        keys(&mut vi, "j0w");
        assert_eq!(vi.cursor.line, 2);
    }
}
