//! Строка поиска по истории.

use dimaster_grid::{Match, Terminal, search_pattern};

use super::Press;

#[derive(Debug, Default)]
pub(crate) struct Search {
    pub(crate) query: String,
    pub(crate) regex: bool,
    pub(crate) matches: Vec<Match>,
    pub(crate) current: usize,
}

/// Что делать окну после нажатия.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Close,
    /// Показать текущее совпадение.
    Show,
}

impl Search {
    /// Пересчитывает совпадения; текущим становится самое свежее.
    pub(crate) fn update(&mut self, term: &Terminal) {
        self.matches = search_pattern(&self.query, self.regex)
            .map(|pattern| term.search(&pattern))
            .unwrap_or_default();
        self.current = self.matches.len().saturating_sub(1);
    }

    // Enter идёт к более старым, Shift+Enter — к новым, по кругу
    fn step(&mut self, older: bool) {
        let count = self.matches.len();
        if count > 0 {
            let delta = if older { count - 1 } else { 1 };
            self.current = (self.current + delta) % count;
        }
    }

    pub(crate) fn press(&mut self, press: Press, term: &Terminal) -> Outcome {
        match press {
            Press::Escape => return Outcome::Close,
            Press::Enter { shift } => self.step(!shift),
            Press::Up => self.step(true),
            Press::Down => self.step(false),
            Press::Backspace => {
                self.query.pop();
                self.update(term);
            }
            Press::Ctrl('r') => {
                self.regex = !self.regex;
                self.update(term);
            }
            Press::Char(ch) => {
                self.query.push(ch);
                self.update(term);
            }
            _ => {}
        }
        Outcome::Show
    }

    pub(crate) fn status(&self) -> String {
        let position = if self.matches.is_empty() {
            0
        } else {
            self.current + 1
        };
        let kind = if self.regex { "regex" } else { "текст" };
        format!(
            " Поиск: {}_  {position}/{}  [{kind}, Ctrl+R] ",
            self.query,
            self.matches.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use dimaster_grid::Size;

    use super::*;

    #[test]
    fn typing_searches_and_enter_walks_back() {
        let mut term = Terminal::new(Size { rows: 4, cols: 10 });
        term.advance(b"foo\r\nbar\r\nfoo");
        let mut search = Search::default();
        for ch in "foo".chars() {
            search.press(Press::Char(ch), &term);
        }
        assert_eq!((search.matches.len(), search.current), (2, 1));
        search.press(Press::Enter { shift: false }, &term);
        assert_eq!(search.current, 0);
        search.press(Press::Enter { shift: false }, &term);
        assert_eq!(search.current, 1);
        assert_eq!(search.press(Press::Escape, &term), Outcome::Close);
    }
}
