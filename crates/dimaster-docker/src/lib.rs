//! TUI для контейнеров: `dimaster docker`.
//!
//! Всё делается через клиент `docker` без shell: локально он учитывает текущий
//! context, на сервере вызывается как `docker -H ssh://имя`. Запросы идут в
//! фоновых потоках, поэтому медленный демон не замораживает экран.

mod actions;
mod app;
mod host;
mod keys;
mod kind;
mod logs;
mod process;
mod render;
mod terminal;
mod worker;

pub use app::App;
pub use host::Host;
pub use process::capture;
pub use terminal::run;
