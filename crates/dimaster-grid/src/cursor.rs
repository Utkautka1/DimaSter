//! Позиция печати и всё, что сохраняется вместе с ней (DECSC).

use crate::Style;
use crate::charset::Charsets;

/// Курсор экрана.
#[derive(Debug, Default, Clone, Copy)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
    pub style: Style,
    // отложенный перенос: символ в последней колонке не двигает курсор сразу
    pub(crate) pending_wrap: bool,
    pub(crate) charsets: Charsets,
}
