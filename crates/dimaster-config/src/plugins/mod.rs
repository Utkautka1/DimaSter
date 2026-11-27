//! Плагины самого терминала: `plugins/*.lua` в каталоге настроек.
//!
//! Плагин не получает доступ к внутренностям: он читает снимок состояния и
//! складывает желаемые действия в очередь, которую выполняет приложение.
//! Зависший плагин прерывается по времени, чтобы не заморозить окно.

mod api;
mod draw;
mod hooks;
mod status;

use dimaster_i18n::t;
use dimaster_i18n::tr;
use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use mlua::{Function, HookTriggers, Lua, Table, VmState};

use crate::context;

pub use draw::{Paint, Shape};
pub use status::{BarStyle, Segment, StatusBar, StatusLine};

// дольше этого один вызов плагина выполняться не должен
const BUDGET: Duration = Duration::from_millis(100);

/// Что плагин может узнать о терминале в момент вызова.
#[derive(Debug, Default, Clone)]
pub struct Snapshot {
    pub title: String,
    pub cwd: String,
    pub selection: String,
    pub screen: String,
    /// Последние строки истории вместе с экраном; пусто для бара и рисования.
    pub history: String,
}

/// Действие, запрошенное плагином.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Send(String),
    Notify {
        title: String,
        body: String,
    },
    Action(String),
    /// Таблица настроек литералом Lua: накладывается поверх файлов.
    Set(String),
    /// Забыть всё, что установили плагины.
    Reset,
}

#[derive(Default)]
struct Shared {
    snapshot: RefCell<Snapshot>,
    effects: RefCell<Vec<Effect>>,
}

/// Загруженные плагины.
pub struct Plugins {
    registry: Table,
    shared: Rc<Shared>,
    deadline: Rc<Cell<Instant>>,
    // интерпретатор должен жить, пока жив реестр
    lua: Lua,
}

fn limit_time(lua: &Lua, deadline: Rc<Cell<Instant>>) -> mlua::Result<()> {
    let triggers = HookTriggers {
        every_nth_instruction: Some(10_000),
        ..HookTriggers::default()
    };
    lua.set_global_hook(triggers, move |_, _| {
        if Instant::now() > deadline.get() {
            return Err(mlua::Error::runtime(tr("плагин выполняется слишком долго")));
        }
        Ok(VmState::Continue)
    })
}

fn scripts(directory: &Path) -> Vec<std::path::PathBuf> {
    let entries = std::fs::read_dir(directory.join("plugins"))
        .into_iter()
        .flatten()
        .flatten();
    let mut found: Vec<_> = entries
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "lua"))
        .collect();
    found.sort();
    found
}

// у каждого файла свой запас времени; сломанный не мешает остальным
fn run_scripts(lua: &Lua, directory: &Path, deadline: &Cell<Instant>) -> Vec<String> {
    let mut errors = Vec::new();
    for path in scripts(directory) {
        deadline.set(Instant::now() + BUDGET);
        let name = path.display().to_string();
        let result = std::fs::read_to_string(&path)
            .map_err(mlua::Error::external)
            .and_then(|source| lua.load(source).set_name(name.as_str()).exec());
        errors.extend(result.err().map(|e| format!("{name}: {e}")));
    }
    errors
}

// пользовательский statusbar.lua или встроенный
fn run_statusbar(lua: &Lua, directory: &Path) -> Result<(), String> {
    let path = directory.join("statusbar.lua");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| include_str!("../scaffold/files/statusbar.lua").to_owned());
    lua.load(source)
        .set_name("statusbar.lua")
        .exec()
        .map_err(|e| format!("{}: {e}", path.display()))
}

// без файла рисовать нечего
fn run_draw(lua: &Lua, directory: &Path) -> Result<(), String> {
    let path = directory.join("draw.lua");
    let Ok(source) = std::fs::read_to_string(&path) else {
        return Ok(());
    };
    lua.load(source)
        .set_name("draw.lua")
        .exec()
        .map_err(|e| format!("{}: {e}", path.display()))
}

impl Plugins {
    // таблица `info` для statusbar.lua и draw.lua
    fn info(&self, facts: &[(String, String)]) -> mlua::Result<Table> {
        let info = self.lua.create_table()?;
        for (name, value) in facts {
            info.set(name.as_str(), value.as_str())?;
        }
        Ok(info)
    }

    /// Возвращает плагины и сообщения о тех, что не загрузились.
    pub fn load(directory: &Path) -> mlua::Result<(Self, Vec<String>)> {
        let lua = Lua::new();
        context::install(&lua, directory)?;
        let shared = Rc::new(Shared::default());
        let deadline = Rc::new(Cell::new(Instant::now() + BUDGET));
        limit_time(&lua, deadline.clone())?;
        api::install(&lua, &shared)?;
        let registry: Table = lua
            .load(include_str!("prelude.lua"))
            .set_name("prelude")
            .eval()?;
        let mut errors = run_scripts(&lua, directory, &deadline);
        deadline.set(Instant::now() + BUDGET);
        errors.extend(run_statusbar(&lua, directory).err());
        errors.extend(run_draw(&lua, directory).err());
        Ok((
            Self {
                registry,
                shared,
                deadline,
                lua,
            },
            errors,
        ))
    }

    // свежий снимок и новый запас времени перед вызовом Lua
    fn prepare_call(&self, snapshot: Snapshot) {
        *self.shared.snapshot.borrow_mut() = snapshot;
        self.deadline.set(Instant::now() + BUDGET);
    }

    fn call(&self, method: &str, arguments: impl mlua::IntoLuaMulti, snapshot: Snapshot) {
        self.prepare_call(snapshot);
        let result = self
            .registry
            .get::<Function>(method)
            .and_then(|f| f.call::<()>(arguments));
        if let Err(e) = result {
            log::warn!("{}", t!("Плагины: {error}", error = e));
        }
    }

    /// Сообщает о событии: `startup`, `title`, `cwd`, `focus`, `notification`.
    pub fn emit(&self, event: &str, value: &str, snapshot: Snapshot) -> Vec<Effect> {
        self.call("emit", (event, value), snapshot);
        self.shared.effects.take()
    }

    pub fn run(&self, command: &str, snapshot: Snapshot) -> Vec<Effect> {
        self.call("run", command, snapshot);
        self.shared.effects.take()
    }

    /// Команды плагинов: имя и название для палитры.
    pub fn commands(&self) -> Vec<(String, String)> {
        let list = self
            .registry
            .get::<Function>("list")
            .and_then(|f| f.call::<Vec<Vec<String>>>(()));
        list.unwrap_or_default()
            .into_iter()
            .filter_map(|pair| Some((pair.first()?.clone(), pair.get(1)?.clone())))
            .collect()
    }
}
