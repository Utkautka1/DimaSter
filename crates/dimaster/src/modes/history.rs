//! Time Machine: команды этого окна со снимками их вывода.

use dimaster_grid::Finished;

use super::Press;

#[derive(Debug, Default)]
pub(crate) struct History {
    // новые сначала
    pub(crate) entries: Vec<Finished>,
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
    pub(crate) fn press(&mut self, press: Press) -> Outcome {
        let last = self.entries.len().saturating_sub(1);
        let current = self.entries.get(self.selected);
        match press {
            Press::Escape | Press::Char('q') => return Outcome::Close,
            Press::Up | Press::Char('k') => self.selected = self.selected.saturating_sub(1),
            Press::Down | Press::Char('j') => self.selected = (self.selected + 1).min(last),
            Press::Enter { .. } => {
                return current.map_or(Outcome::Close, |e| Outcome::Insert(e.command.clone()));
            }
            Press::Char('y') => {
                return current
                    .map_or(Outcome::Close, |e| Outcome::CopyOutput(e.output.join("\n")));
            }
            _ => {}
        }
        Outcome::Stay
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
            selected: 0,
        };
        history.press(Press::Down);
        history.press(Press::Down);
        assert_eq!(
            history.press(Press::Enter { shift: false }),
            Outcome::Insert("old".into())
        );
        assert_eq!(
            history.press(Press::Char('y')),
            Outcome::CopyOutput("o\np".into())
        );
    }
}
