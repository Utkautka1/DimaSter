//! Гиперссылки OSC 8: текст ссылки и её адрес могут различаться.
//!
//! Клетка хранит только номер, сами адреса лежат в таблице: так ячейки не
//! раздуваются, а одинаковые ссылки не дублируются.

use std::collections::{HashMap, HashSet};

use crate::terminal::State;
use crate::untrusted::clean;

#[derive(Debug, Default)]
pub(crate) struct Links {
    uris: Vec<String>,
    numbers: HashMap<String, u16>,
    // номера, освобождённые уборкой
    free: Vec<u16>,
    // сколько переполнений пропустить до следующей уборки
    pause: usize,
}

impl Links {
    fn number(&mut self, uri: &str) -> u16 {
        if let Some(&number) = self.numbers.get(uri) {
            return number;
        }
        let number = match self.free.pop() {
            Some(number) => number,
            None if self.uris.len() < dimaster_limits::HYPERLINKS.size() => {
                self.uris.push(String::new());
                self.uris.len() as u16
            }
            None => return 0,
        };
        uri.clone_into(&mut self.uris[usize::from(number) - 1]);
        self.numbers.insert(uri.to_owned(), number);
        number
    }

    // пора ли освобождать номера ради нового адреса
    fn wants_sweep(&mut self, uri: &str) -> bool {
        let full = self.free.is_empty() && self.uris.len() >= dimaster_limits::HYPERLINKS.size();
        if !full || self.numbers.contains_key(uri) {
            return false;
        }
        let ready = self.pause == 0;
        self.pause = self.pause.saturating_sub(1);
        ready
    }

    fn sweep(&mut self, used: &HashSet<u16>) {
        self.numbers.retain(|_, number| used.contains(number));
        let unused = (1..=self.uris.len() as u16).filter(|n| !used.contains(n));
        self.free = unused.collect();
        if self.free.len() < dimaster_limits::HYPERLINK_SWEEP_PAUSE.size() / 4 {
            self.pause = dimaster_limits::HYPERLINK_SWEEP_PAUSE.size();
        }
    }

    pub(crate) fn uri(&self, number: u16) -> Option<&str> {
        self.uris
            .get(usize::from(number).checked_sub(1)?)
            .map(String::as_str)
    }
}

impl State {
    // номера, на которые ещё ссылаются экраны, история и курсоры
    fn used_links(&self) -> HashSet<u16> {
        let screens = [&self.screen, &self.other];
        let cells = screens.iter().flat_map(|s| &s.rows).flat_map(|r| &r.cells);
        let history = screens.iter().flat_map(|s| &s.history);
        let cursors = self.saved.iter().chain([&self.cursor]);
        cells
            .map(|c| c.style.link)
            .chain(history.flat_map(|r| r.links()))
            .chain(cursors.map(|c| c.style.link))
            .collect()
    }

    /// `OSC 8;параметры;адрес`; пустой адрес закрывает ссылку.
    pub(crate) fn hyperlink(&mut self, value: &str) {
        let Some((_, uri)) = value.split_once(';') else {
            return;
        };
        let uri = clean(uri);
        // прежняя ссылка курсора больше не нужна и не должна держать номер
        self.cursor.style.link = 0;
        if self.links.wants_sweep(&uri) {
            let used = self.used_links();
            self.links.sweep(&used);
        }
        self.cursor.style.link =
            match uri.is_empty() || uri.len() > dimaster_limits::HYPERLINK_BYTES.size() {
                true => 0,
                false => self.links.number(&uri),
            };
    }
}
