//! Шрифты: поиск в системе, шейпинг и растеризация глифов.
//!
//! Растеризатор контуров свой, шейпинг делает `rustybuzz` (лигатуры, арабская
//! вязь), цветные эмодзи берутся из PNG-растров таблицы CBDT.

mod color;
mod colr;
mod discover;
mod raster;
mod shape;
mod sprite;
mod system;

pub use raster::Bitmap;
pub use shape::{GlyphId, ShapedGlyph};
pub use system::{FontError, FontSystem, Metrics};
