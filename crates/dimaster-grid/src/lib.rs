//! Состояние экрана терминала: сетка ячеек, курсор, история и режимы.
//!
//! [`Terminal`] принимает байты программы и меняет экран. Ответы на запросы
//! (DA, DSR) копятся в буфере — их надо отправить обратно в PTY, иначе
//! fish и nvim будут ждать их при старте.

mod cell;
mod charset;
mod cursor;
mod dispatch;
mod edit;
mod modes;
mod osc;
mod row;
mod screen;
mod sgr;
mod terminal;

pub use cell::{Cell, Color, Flags, Style};
pub use cursor::Cursor;
pub use modes::Modes;
pub use row::{PromptMark, Row};
pub use screen::Screen;
pub use terminal::{Size, Terminal};
