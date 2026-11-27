//! Просмотр таблиц в терминале: `dimaster view файл…`.
//!
//! Состояние ([`App`]) не знает о терминале: клавиши меняют его, а отрисовка
//! возвращает строки — поэтому всё поведение проверяется тестами. Цвета —
//! только 16 цветов ANSI, так что таблица одета в тему терминала.

mod app;
mod command;
mod fuzzy;
mod keys;
mod render;
mod sheet;
mod terminal;

pub use app::App;
pub use terminal::run;
