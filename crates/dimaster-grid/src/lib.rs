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
mod graphics;
mod keyboard;
mod modes;
mod osc;
mod reflow;
mod report;
mod row;
mod screen;
mod selection;
mod sgr;
mod terminal;

pub use cell::{Cell, Color, Flags, Style};
pub use cursor::Cursor;
pub use graphics::{MAX_IMAGE_ROWS, Picture, Placement};
pub use modes::{CursorShape, Modes};
pub use report::DefaultColors;
pub use row::{PromptMark, Row};
pub use screen::Screen;
pub use selection::{Point, Selection, SelectionKind, Span};
pub use terminal::{Size, Terminal};
