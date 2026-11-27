//! Time Machine: команды этого окна со снимками их вывода.

use dimaster_grid::Finished;

use super::Press;
use super::picker::fuzzy_score;

#[derive(Debug, Default)]
pub(crate) struct History {
    // новые сначала
    pub(crate) entries: Vec<Finished>,
    pub(crate) query: String,
    pub(crate) selected: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Stay,
    Close,
    /// Вставить команду в строку ввода, не выполняя.
    Insert(String),
    CopyOutput(String),
}

impl History {
    pub(crate) fn filtered(&self) -> Vec<(usize, &Finished)> {
        let mut items: Vec<(i32, usize, &Finished)> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| Some((fuzzy_score(&self.query, &e.command)?, i, e)))
            .collect();
        if !self.query.is_empty() {
            items.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        }
        items.into_iter().map(|(_, i, e)| (i, e)).collect()
    }

    pub(crate) fn selected_entry(&self) -> Option<&Finished> {
        self.filtered().get(self.selected).map(|(_, e)| *e)
    }

    fn step(&mut self, next: usize) -> Outcome {
        self.selected = next;
        Outcome::Stay
    }

    pub(crate) fn press(&mut self, press: Press) -> Outcome {
        let last = self.filtered().len().saturating_sub(1);
        match press {
            Press::Escape | Press::Ctrl('c') => Outcome::Close,
            Press::Up | Press::Ctrl('k' | 'p') => self.step(self.selected.saturating_sub(1)),
            Press::Down | Press::Ctrl('j' | 'n') => self.step((self.selected + 1).min(last)),
            Press::Enter { .. } => self
                .selected_entry()
                .map_or(Outcome::Close, |e| Outcome::Insert(e.command.clone())),
            Press::Ctrl('y') => self
                .selected_entry()
                .map_or(Outcome::Close, |e| Outcome::CopyOutput(e.output.join("\n"))),
            Press::Backspace => {
                self.query.pop();
                self.step(0)
            }
            Press::Char(ch) => {
                self.query.push(ch);
                self.step(0)
            }
            _ => Outcome::Stay,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn navigates_inserts_and_copies() {
        let entry = |command: &str| Finished {
            command: command.into(),
            exit: 0,
            duration: Duration::ZERO,
            output: vec!["o".into(), "p".into()],
        };
        let mut history = History {
            entries: vec![entry("new"), entry("old")],
            query: String::new(),
            selected: 0,
        };
        assert_eq!(history.press(Press::Down), Outcome::Stay);
        assert_eq!(
            history.press(Press::Enter { shift: false }),
            Outcome::Insert("old".into())
        );
        assert_eq!(
            history.press(Press::Ctrl('y')),
            Outcome::CopyOutput("o\np".into())
        );
        assert_eq!(history.press(Press::Char('n')), Outcome::Stay);
        assert_eq!(
            history.press(Press::Enter { shift: false }),
            Outcome::Insert("new".into())
        );
    }
}
