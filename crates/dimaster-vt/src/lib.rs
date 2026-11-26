//! Разбор потока байтов терминала в команды.
//!
//! Автомат повторяет модель Пола Уильямса для DEC-совместимых терминалов,
//! но с двумя отличиями: UTF-8 декодируется прямо здесь, а APC не
//! выбрасывается — через него работает графический протокол Kitty.
//! Парсер ничего не знает об экране: команды получает реализация [`Perform`].

mod params;
mod parser;
mod perform;
mod strings;
mod utf8;

pub use params::Params;
pub use parser::Parser;
pub use perform::{Perform, Sequence};
