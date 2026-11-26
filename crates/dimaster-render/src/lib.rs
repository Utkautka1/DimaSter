//! Отрисовка терминала на GPU.
//!
//! Кадр — это список прямоугольников одного вида (фон, глиф из атласа, линия),
//! который рисуется одним instanced-вызовом. Сборка списка отделена от wgpu,
//! поэтому проверяется тестами без видеокарты.

mod atlas;
mod bidi;
mod frame;
mod gpu;
mod palette;
mod pipeline;
mod preedit;
mod text;

pub use frame::{PADDING, Viewport};
pub use gpu::{RenderError, Renderer};
// приложению нужно создать поверхность окна той же версией wgpu
pub use wgpu;
