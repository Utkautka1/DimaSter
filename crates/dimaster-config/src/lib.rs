//! Настройки терминала: Lua-конфиг, темы и импорт из других терминалов.
//!
//! Конфиг — обычная Lua-программа, возвращающая таблицу, поэтому в нём можно
//! писать условия и функции. Крейт отдаёт приложению уже проверенные данные:
//! Lua не протекает дальше этой границы.

mod color;
mod config;
mod context;
mod cursor;
mod fetch;
mod fetch_picture;
pub mod import;
mod keys;
mod lang;
mod load;
mod load_keys;
mod load_smart;
mod load_ui;
mod plugins;
mod programs;
mod scaffold;
mod skins;
mod smart;
mod snippets;
mod theme;
mod ui;
mod widgets;
mod window;

pub use color::parse_color;
pub use config::{Config, CopyOnSelect, CursorStyle};
pub use cursor::CursorLook;
pub use fetch::{DEFAULT_FETCH, Layout, fetch_color, run_fetch};
pub use fetch_picture::{PicturePosition, PictureStyle};
pub use keys::{Action, Binding, Chord, Mods};
pub use lang::translations;
pub use load::{ConfigError, config_dir, load, load_with, parse, parse_with};
pub use plugins::{BarStyle, Segment, StatusBar, StatusLine};
pub use plugins::{Effect, Plugins, Snapshot};
pub use plugins::{Paint, Shape};
pub use programs::{Bell, Programs};
pub use scaffold::{File, scaffold};
pub use skins::{Skin, skin_literal, skins};
pub use smart::Smart;
pub use snippets::Snippet;
pub use theme::{Theme, builtin_names};
pub use ui::{
    Align, Anchor, HistoryStyle, Marks, MonitorStyle, NoticeStyle, PaletteStyle, PanelStyle, Strip,
};
pub use ui::{Texts, TitleButton, TitleStyle, Ui, WindowButton};
pub use widgets::Widgets;
pub use window::{Decorations, Effects, WindowConfig};
