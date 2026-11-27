//! Нажатия, которые терминал обрабатывает за shell: стрелки по истории,
//! сокращения и меню дополнения.
//!
//! Строка ввода читается с экрана, а меняется обычными нажатиями, поэтому
//! работает в любом shell с метками команд. Не уверены — клавиша уходит shell.

use winit::keyboard::Key;

use crate::complete;
use crate::modes::{Mode, picker};
use crate::view::View;

// Ctrl+E, Ctrl+U: строку целиком очищают readline, zle и fish
const CLEAR_LINE: &[u8] = b"\x05\x15";

/// Положение в истории, пока пользователь её листает.
#[derive(Debug, Default)]
pub(crate) struct Walk {
    prefix: String,
    // 0 — набранное, дальше найденные команды от новых к старым
    index: usize,
    shown: String,
    // совпадения, собранные в начале листания: файл shell читается один раз
    found: Vec<String>,
}

impl Walk {
    /// Следующая строка или `None`, когда листать нечего.
    fn step(&mut self, input: &str, found: &[String], back: bool) -> Option<String> {
        let index = match back {
            true => (self.index + 1).min(found.len()),
            false => self.index.checked_sub(1)?,
        };
        if index == self.index && self.shown == input {
            return None;
        }
        self.index = index;
        self.shown = match index {
            0 => self.prefix.clone(),
            _ => found.get(index - 1)?.clone(),
        };
        Some(self.shown.clone())
    }
}

impl View {
    // набранное в приглашении и стоит ли перед курсором пробел
    fn typed_line(&self) -> Option<(String, bool)> {
        if self.term.is_alternate() || !crate::syntax::editing_command(&self.term) {
            return None;
        }
        let input = crate::syntax::input_before_cursor(&self.term)?;
        let spaced = crate::autopair::neighbors(&self.term).0 == Some(' ');
        Some((input, spaced))
    }

    // сначала новые из этого окна и DimaSter, затем история самого shell
    fn past_commands(&self, prefix: &str) -> Vec<String> {
        let live: Vec<String> = self.term.commands().map(|c| c.command.clone()).collect();
        let mut older = match self.config.native.shell_history {
            true => crate::shell_history::read(&crate::settings::shell(&self.config)[0]),
            false => Vec::new(),
        };
        older.extend(live);
        self.history_store.matching(prefix, &older)
    }

    fn history_bytes(&mut self, input: &str, back: bool) -> Option<Vec<u8>> {
        // чужая или изменённая строка начинает поиск заново
        if self.assist.walk.shown != input {
            if !back {
                return None;
            }
            self.assist.walk = Walk {
                prefix: input.to_owned(),
                index: 0,
                shown: input.to_owned(),
                found: self.past_commands(input),
            };
        }
        let found = std::mem::take(&mut self.assist.walk.found);
        if found.is_empty() {
            return None;
        }
        let line = self.assist.walk.step(input, &found, back);
        self.assist.walk.found = found;
        let Some(line) = line else {
            return Some(Vec::new());
        };
        Some([CLEAR_LINE, line.as_bytes()].concat())
    }

    fn abbreviation_bytes(&self, input: &str) -> Option<Vec<u8>> {
        let found = self.config.native.abbreviations.iter();
        let (name, full) = found.into_iter().find(|a| a.0 == input)?;
        let mut bytes = vec![0x7f; name.chars().count()];
        bytes.extend_from_slice(format!("{full} ").as_bytes());
        Some(bytes)
    }

    // один вариант вставляется сразу, несколько открывают меню
    fn complete_bytes(&mut self, input: &str, spaced: bool) -> Option<Vec<u8>> {
        let found = complete::find(self, input, spaced)?;
        if let [only] = found.items.as_slice() {
            return Some(complete::replace(&found.word, only));
        }
        self.mode = Mode::Picker(picker::Picker {
            kind: picker::Kind::Complete,
            items: found.items,
            values: vec![found.word],
            query: String::new(),
            selected: 0,
        });
        self.window.request_redraw();
        Some(Vec::new())
    }

    /// Байты для программы, если нажатие обработал терминал.
    pub(crate) fn native_bytes(&mut self, key: &Key) -> Option<Vec<u8>> {
        let native = &self.config.native;
        let back = self.pressed(key, &native.history_back);
        let walk = native.history_search && (back || self.pressed(key, &native.history_forward));
        let tab = native.complete && self.pressed(key, &native.complete_keys);
        let space = matches!(key, Key::Named(winit::keyboard::NamedKey::Space))
            && self.modifiers.is_empty();
        if !(walk || tab || space) {
            return None;
        }
        let (input, spaced) = self.typed_line()?;
        match (walk, tab) {
            (true, _) => self.history_bytes(&input, back),
            (_, true) => self.complete_bytes(&input, spaced),
            _ if !spaced => self.abbreviation_bytes(&input),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_goes_back_and_returns_to_typed() {
        let found = ["git push".to_owned(), "git pull".to_owned()];
        let mut walk = Walk {
            prefix: "git".into(),
            index: 0,
            shown: "git".into(),
            found: Vec::new(),
        };
        assert_eq!(walk.step("git", &found, true).as_deref(), Some("git push"));
        assert_eq!(
            walk.step("git push", &found, true).as_deref(),
            Some("git pull")
        );
        assert_eq!(walk.step("git pull", &found, true), None);
        assert_eq!(
            walk.step("git pull", &found, false).as_deref(),
            Some("git push")
        );
        assert_eq!(walk.step("git push", &found, false).as_deref(), Some("git"));
        assert_eq!(walk.step("git", &found, false), None);
    }
}
