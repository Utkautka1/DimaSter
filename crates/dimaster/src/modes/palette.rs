//! Палитра команд: все действия терминала с поиском по названию.

use dimaster_config::Action;

use super::Press;

const COMMANDS: [(&str, Action); 11] = [
    ("Копировать", Action::Copy),
    ("Вставить", Action::Paste),
    ("Поиск по истории", Action::Search),
    ("Vi-режим", Action::ViMode),
    ("Увеличить шрифт", Action::FontIncrease),
    ("Уменьшить шрифт", Action::FontDecrease),
    ("Сбросить шрифт", Action::FontReset),
    ("В начало истории", Action::ScrollTop),
    ("В конец истории", Action::ScrollBottom),
    ("Перечитать конфиг", Action::ReloadConfig),
    ("Вставить выделенное (primary)", Action::PastePrimary),
];

#[derive(Debug, Default)]
pub(crate) struct Palette {
    // команды плагинов: имя и название
    pub(crate) plugins: Vec<(String, String)>,
    pub(crate) query: String,
    pub(crate) selected: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Stay,
    Close,
    Run(Action),
}

impl Palette {
    pub(crate) fn items(&self) -> Vec<(String, Action)> {
        let query = self.query.to_lowercase();
        let own = COMMANDS
            .iter()
            .map(|(name, action)| ((*name).to_owned(), action.clone()));
        let extra = self
            .plugins
            .iter()
            .map(|(name, title)| (title.clone(), Action::Plugin(name.clone())));
        own.chain(extra)
            .filter(|c| c.0.to_lowercase().contains(&query))
            .collect()
    }

    pub(crate) fn press(&mut self, press: Press) -> Outcome {
        let count = self.items().len();
        match press {
            Press::Escape => return Outcome::Close,
            Press::Enter { .. } => {
                let chosen = self.items().get(self.selected).map(|c| c.1.clone());
                return chosen.map_or(Outcome::Close, Outcome::Run);
            }
            Press::Up => self.selected = (self.selected + count.max(1) - 1) % count.max(1),
            Press::Down => self.selected = (self.selected + 1) % count.max(1),
            Press::Backspace => {
                self.query.pop();
                self.selected = 0;
            }
            Press::Char(ch) => {
                self.query.push(ch);
                self.selected = 0;
            }
            _ => {}
        }
        Outcome::Stay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_and_runs() {
        let mut palette = Palette::default();
        for ch in "шрифт".chars() {
            palette.press(Press::Char(ch));
        }
        assert_eq!(palette.items().len(), 3);
        palette.press(Press::Down);
        let outcome = palette.press(Press::Enter { shift: false });
        assert_eq!(outcome, Outcome::Run(Action::FontDecrease));
    }
}
