//! Настройки терминала: Lua-конфиг, темы и импорт из других терминалов.
//!
//! Конфиг — обычная Lua-программа, возвращающая таблицу, поэтому в нём можно
//! писать условия и функции. Крейт отдаёт приложению уже проверенные данные:
//! Lua не протекает дальше этой границы.

mod color;
mod config;
mod context;
pub mod import;
mod keys;
mod load;
mod plugins;
mod scaffold;
mod theme;
mod window;

pub use color::parse_color;
pub use config::{Config, CursorStyle};
pub use keys::{Action, Binding, Mods};
pub use load::{ConfigError, config_dir, load, parse};
pub use plugins::{Effect, Plugins, Snapshot};
pub use scaffold::{File, scaffold};
pub use theme::{Theme, builtin_names};
pub use window::{Decorations, Effects, WindowConfig};
