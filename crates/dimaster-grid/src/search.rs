//! Поиск по истории и распознавание ссылок.
//!
//! Строки, соединённые мягким переносом, ищутся как одна, иначе длинный адрес
//! на границе строки не нашёлся бы. Движок `regex` работает за линейное время,
//! поэтому неудачный шаблон не может подвесить терминал.

use regex::{Regex, RegexBuilder};

use crate::{Flags, Point, Terminal};

/// Совпадение: обе границы включительно.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub start: Point,
    pub end: Point,
}

// логическая строка и позиция в сетке для каждого символа
struct Logical {
    text: String,
    points: Vec<(usize, Point)>,
}

impl Logical {
    fn point(&self, byte: usize) -> Option<Point> {
        let index = self
            .points
            .partition_point(|p| p.0 <= byte)
            .checked_sub(1)?;
        Some(self.points[index].1)
    }

    fn matched(&self, start: usize, end: usize) -> Option<Match> {
        Some(Match {
            start: self.point(start)?,
            end: self.point(end.checked_sub(1)?)?,
        })
    }
}

/// Шаблон поиска: обычный текст или регулярное выражение.
/// Регистр учитывается, только если в запросе есть заглавные буквы.
pub fn search_pattern(query: &str, regex: bool) -> Option<Regex> {
    if query.is_empty() {
        return None;
    }
    let source = if regex {
        query.to_owned()
    } else {
        regex::escape(query)
    };
    RegexBuilder::new(&source)
        .case_insensitive(!query.chars().any(char::is_uppercase))
        .size_limit(1 << 20)
        .build()
        .ok()
}

// адреса и пути к файлам
const LINK: &str =
    r#"(?:https?://|file://|mailto:)[^\s<>"'`]+|(?:~|\.{1,2})?/[\w.\-+@%/]+(?::\d+)?"#;
// знаки в конце предложения не входят в адрес
const TRAILING: [char; 7] = ['.', ',', ':', ';', ')', ']', '}'];

impl Terminal {
    /// Номер первой и следующей за последней строки (с историей).
    pub fn line_range(&self) -> std::ops::Range<u64> {
        let screen = self.screen();
        screen.evicted..screen.evicted + (screen.history.len() + screen.rows.len()) as u64
    }

    // собирает логическую строку, начинающуюся с `line`; возвращает следующую
    fn logical(&self, mut line: u64) -> (Logical, u64) {
        let mut logical = Logical {
            text: String::new(),
            points: Vec::new(),
        };
        while let Some(row) = self.line(line) {
            for (col, cell) in row.cells.iter().enumerate() {
                if !cell.style.flags.contains(Flags::WIDE_TAIL) {
                    logical
                        .points
                        .push((logical.text.len(), Point { line, col }));
                    logical.text.push(cell.ch);
                }
            }
            line += 1;
            if !row.wrapped {
                break;
            }
        }
        (logical, line)
    }

    /// Все совпадения сверху вниз.
    pub fn search(&self, pattern: &Regex) -> Vec<Match> {
        let range = self.line_range();
        let mut found = Vec::new();
        let mut line = range.start;
        while line < range.end {
            let (logical, next) = self.logical(line);
            let matches = pattern.find_iter(&logical.text).filter(|m| !m.is_empty());
            found.extend(matches.filter_map(|m| logical.matched(m.start(), m.end())));
            line = next.max(line + 1);
        }
        found
    }

    // начало логической строки, содержащей `line`
    fn logical_start(&self, mut line: u64) -> u64 {
        while line > self.line_range().start && self.line(line - 1).is_some_and(|r| r.wrapped) {
            line -= 1;
        }
        line
    }

    /// Все ссылки и пути в логических строках, задевающих `lines`.
    pub fn links_in(&self, lines: std::ops::Range<u64>) -> Vec<(Match, String)> {
        let Ok(pattern) = Regex::new(LINK) else {
            return Vec::new();
        };
        let mut found = self.hyperlinks_in(lines.clone());
        let mut line = self.logical_start(lines.start);
        while line < lines.end.min(self.line_range().end) {
            let (logical, next) = self.logical(line);
            found.extend(pattern.find_iter(&logical.text).filter_map(|hit| {
                let text = hit.as_str().trim_end_matches(TRAILING);
                let range = logical.matched(hit.start(), hit.start() + text.len())?;
                // текст внутри гиперссылки OSC 8 не повторяется отдельной целью
                let fresh = self.link_number(range.start) == 0;
                (fresh && lines.contains(&range.start.line)).then(|| (range, text.to_owned()))
            }));
            line = next.max(line + 1);
        }
        found
    }

    fn link_number(&self, point: Point) -> u16 {
        let cell = self.line(point.line).and_then(|r| r.cells.get(point.col));
        cell.map_or(0, |c| c.style.link)
    }

    // гиперссылка OSC 8 под точкой: все соседние клетки с тем же номером
    fn hyperlink_at(&self, point: Point) -> Option<(Match, String)> {
        let number = self.link_number(point);
        let uri = self.state.links.uri(number)?.to_owned();
        let same = |col| self.link_number(Point { col, ..point }) == number;
        let first = (0..point.col)
            .rev()
            .take_while(|&c| same(c))
            .last()
            .unwrap_or(point.col);
        let last = (point.col + 1..)
            .take_while(|&c| same(c))
            .last()
            .unwrap_or(point.col);
        let at = |col| Point { col, ..point };
        Some((
            Match {
                start: at(first),
                end: at(last),
            },
            uri,
        ))
    }

    // начала всех гиперссылок OSC 8 в строках
    fn hyperlinks_in(&self, lines: std::ops::Range<u64>) -> Vec<(Match, String)> {
        let mut found = Vec::new();
        for line in lines {
            let Some(row) = self.line(line) else { continue };
            let mut col = 0;
            while col < row.cells.len() {
                match self.hyperlink_at(Point { line, col }) {
                    Some(link) => {
                        col = link.0.end.col + 1;
                        found.push(link);
                    }
                    None => col += 1,
                }
            }
        }
        found
    }

    /// Адрес или путь под указанной клеткой.
    pub fn link_at(&self, point: Point) -> Option<(Match, String)> {
        if let Some(link) = self.hyperlink_at(point) {
            return Some(link);
        }
        let pattern = Regex::new(LINK).ok()?;
        let (logical, _) = self.logical(self.logical_start(point.line));
        pattern.find_iter(&logical.text).find_map(|found| {
            let text = found.as_str().trim_end_matches(TRAILING);
            let range = logical.matched(found.start(), found.start() + text.len())?;
            (range.start..=range.end)
                .contains(&point)
                .then(|| (range, text.to_owned()))
        })
    }

    /// Выделение, покрывающее совпадение, — для подсветки.
    pub fn match_span(&self, found: Match) -> crate::Span {
        let kind = crate::SelectionKind::Simple;
        self.span(crate::Selection {
            kind,
            anchor: found.start,
            head: found.end,
        })
    }
}
