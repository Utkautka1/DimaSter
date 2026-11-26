//! Отрисовка терминала на GPU.
//!
//! Кадр — это список прямоугольников одного вида (фон, глиф из атласа, линия),
//! который рисуется одним instanced-вызовом. Сборка списка отделена от wgpu,
//! поэтому проверяется тестами без видеокарты.

mod appearance;
mod atlas;
mod bidi;
mod cursor;
mod frame;
mod gpu;
mod images;
mod overlay;
mod palette;
mod pipeline;
mod preedit;
mod text;
mod textures;
mod underline;

pub use appearance::{Appearance, CursorLook};
pub use frame::Viewport;
pub use gpu::{RenderError, Renderer};
pub use overlay::{Label, Rect};
// приложению нужно создать поверхность окна той же версией wgpu
pub use wgpu;
