//! Гиперссылки OSC 8: текст ссылки и её адрес могут различаться.
//!
//! Клетка хранит только номер, сами адреса лежат в таблице: так ячейки не
//! раздуваются, а одинаковые ссылки не дублируются.

use std::collections::HashMap;

use crate::terminal::State;
use crate::untrusted::clean;

// номера укладываются в u16; 0 означает «не ссылка»
const MAX_LINKS: usize = 4096;
const MAX_URI: usize = 2048;

#[derive(Debug, Default)]
pub(crate) struct Links {
    uris: Vec<String>,
    numbers: HashMap<String, u16>,
}

impl Links {
    fn number(&mut self, uri: &str) -> u16 {
        if let Some(&number) = self.numbers.get(uri) {
            return number;
        }
        if self.uris.len() >= MAX_LINKS {
            return 0;
        }
        self.uris.push(uri.to_owned());
        let number = self.uris.len() as u16;
        self.numbers.insert(uri.to_owned(), number);
        number
    }

    pub(crate) fn uri(&self, number: u16) -> Option<&str> {
        self.uris
            .get(usize::from(number).checked_sub(1)?)
            .map(String::as_str)
    }
}

impl State {
    /// `OSC 8;параметры;адрес`; пустой адрес закрывает ссылку.
    pub(crate) fn hyperlink(&mut self, value: &str) {
        let Some((_, uri)) = value.split_once(';') else {
            return;
        };
        let uri = clean(uri);
        self.cursor.style.link = match uri.is_empty() || uri.len() > MAX_URI {
            true => 0,
            false => self.links.number(&uri),
        };
    }
}
