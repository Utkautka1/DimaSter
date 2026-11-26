//! Двунаправленный текст: в какой колонке рисовать каждую клетку строки.
//!
//! Сетка хранит символы в логическом порядке; переставляются только клетки
//! при отрисовке, направление абзаца всегда слева направо.

use dimaster_grid::Cell;
use unicode_bidi::{BidiInfo, Level};

// иврит, арабский и их презентационные формы
fn is_rtl(ch: char) -> bool {
    matches!(ch, '\u{0590}'..='\u{08ff}' | '\u{fb1d}'..='\u{fdff}' | '\u{fe70}'..='\u{feff}')
        || matches!(ch, '\u{10800}'..='\u{10fff}' | '\u{1e800}'..='\u{1efff}')
}

/// `result[логическая колонка] = видимая колонка`; `None`, если RTL в строке нет.
pub(crate) fn visual_columns(cells: &[Cell]) -> Option<Vec<usize>> {
    if !cells.iter().any(|c| is_rtl(c.ch)) {
        return None;
    }
    let text: String = cells.iter().map(|c| c.ch).collect();
    let info = BidiInfo::new(&text, Some(Level::ltr()));
    let paragraph = info.paragraphs.first()?;
    let (levels, runs) = info.visual_runs(paragraph, paragraph.range.clone());
    let starts: Vec<usize> = text.char_indices().map(|(at, _)| at).collect();
    let mut columns: Vec<usize> = (0..cells.len()).collect();
    let mut visual = 0;
    for run in runs {
        let first = starts.partition_point(|&at| at < run.start);
        let last = starts.partition_point(|&at| at < run.end);
        let reversed = levels[run.start].is_rtl();
        for offset in 0..last - first {
            let logical = if reversed {
                last - 1 - offset
            } else {
                first + offset
            };
            columns[logical] = visual;
            visual += 1;
        }
    }
    Some(columns)
}

#[cfg(test)]
mod tests {
    use dimaster_grid::Style;

    use super::*;

    fn map(text: &str) -> Option<Vec<usize>> {
        let cells: Vec<_> = text
            .chars()
            .map(|c| Cell::new(c, Style::default()))
            .collect();
        visual_columns(&cells)
    }

    #[test]
    fn hebrew_run_is_reversed_inside_latin_line() {
        assert_eq!(map("ab אבג cd"), Some(vec![0, 1, 2, 5, 4, 3, 6, 7, 8]));
        assert_eq!(map("plain"), None);
    }
}
