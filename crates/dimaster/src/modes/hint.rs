//! Режим подсказок: буквенные метки на ссылках и путях в видимой области.

use dimaster_grid::{Point, Terminal};

use super::Press;

#[derive(Debug, Clone)]
pub(crate) struct Target {
    pub(crate) label: String,
    pub(crate) at: Point,
    pub(crate) text: String,
}

#[derive(Debug, Default)]
pub(crate) struct Hints {
    pub(crate) targets: Vec<Target>,
    pub(crate) typed: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Stay,
    Close,
    Open(String),
    /// Заглавная буква: скопировать вместо открытия.
    Copy(String),
}

// если целей больше букв, все метки двухбуквенные — иначе выбор неоднозначен
fn label(index: usize, total: usize, letters: &[u8]) -> String {
    let letter = |i: usize| char::from(letters[i % letters.len()]);
    match total > letters.len() {
        true => [letter(index / letters.len()), letter(index)]
            .iter()
            .collect(),
        false => letter(index).to_string(),
    }
}

impl Hints {
    pub(crate) fn collect(term: &Terminal, scroll: usize, letters: &str) -> Self {
        let letters = letters.as_bytes();
        let first = term.visible_line(scroll);
        let lines = first..first + term.screen().rows().len() as u64;
        let mut found = term.links_in(lines);
        found.truncate(letters.len() * letters.len());
        let total = found.len();
        let targets = found
            .into_iter()
            .enumerate()
            .map(|(i, (range, text))| Target {
                label: label(i, total, letters),
                at: range.start,
                text,
            })
            .collect();
        Self {
            targets,
            typed: String::new(),
        }
    }

    pub(crate) fn press(&mut self, press: Press) -> Outcome {
        let Press::Char(ch) = press else {
            return if press == Press::Escape {
                Outcome::Close
            } else {
                Outcome::Stay
            };
        };
        self.typed.push(ch.to_ascii_lowercase());
        if let Some(target) = self.targets.iter().find(|t| t.label == self.typed) {
            let text = target.text.clone();
            return if ch.is_uppercase() {
                Outcome::Copy(text)
            } else {
                Outcome::Open(text)
            };
        }
        let possible = self
            .targets
            .iter()
            .any(|t| t.label.starts_with(&self.typed));
        if possible {
            Outcome::Stay
        } else {
            Outcome::Close
        }
    }
}

#[cfg(test)]
mod tests {
    use dimaster_grid::Size;

    use super::*;

    const LETTERS: &str = "asdfghjklqwertyuiopzxcvbnm";

    #[test]
    fn letters_pick_links() {
        let mut term = Terminal::new(Size { rows: 3, cols: 40 });
        term.advance(b"https://a.example\r\nsee /etc/hosts");
        let mut hints = Hints::collect(&term, 0, LETTERS);
        assert_eq!(hints.targets.len(), 2);
        assert_eq!(
            hints.press(Press::Char('S')),
            Outcome::Copy("/etc/hosts".into())
        );
        hints.typed.clear();
        assert_eq!(
            hints.press(Press::Char('a')),
            Outcome::Open("https://a.example".into())
        );
        assert_eq!(label(27, 40, LETTERS.as_bytes()), "ss");
    }
}
