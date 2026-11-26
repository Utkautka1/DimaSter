//! Шрифты: поиск в системе, метрики ячейки и растеризация глифов.
//!
//! Растеризатор свой; из `ttf-parser` берутся только контуры и таблицы.
//! Цветные эмодзи, лигатуры и сложный шейпинг появятся на этапе типографики.

mod discover;
mod raster;
mod system;

pub use raster::Bitmap;
pub use system::{FontError, FontSystem, GlyphKey, Metrics};
