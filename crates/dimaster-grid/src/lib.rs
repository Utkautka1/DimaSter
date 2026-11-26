//! Состояние экрана терминала: сетка ячеек, курсор, история и режимы.
//!
//! [`Terminal`] принимает байты программы и меняет экран. Ответы на запросы
//! (DA, DSR) копятся в буфере — их надо отправить обратно в PTY, иначе
//! fish и nvim будут ждать их при старте.

mod cell;
mod charset;
mod colors;
mod command;
mod cursor;
mod dispatch;
mod edit;
mod graphics;
mod keyboard;
mod modes;
mod osc;
mod query;
mod reflow;
mod report;
mod row;
mod screen;
mod search;
mod selection;
mod sgr;
mod terminal;
mod untrusted;

pub use cell::{Cell, Color, Flags, Style};
pub use colors::{ColorOverrides, DefaultColors};
pub use command::{Finished, SmartOptions, describe_exit};
pub use cursor::Cursor;
pub use graphics::{MAX_IMAGE_ROWS, Picture, Placement};
pub use modes::{CursorShape, Modes};
pub use regex::Regex;
pub use row::{PromptMark, Row};
pub use screen::Screen;
pub use search::{Match, search_pattern};
pub use selection::{Point, Selection, SelectionKind, Span};
pub use terminal::{Size, Terminal};
