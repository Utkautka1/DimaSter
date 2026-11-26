//! Плагины самого терминала: `plugins/*.lua` в каталоге настроек.
//!
//! Плагин не получает доступ к внутренностям: он читает снимок состояния и
//! складывает желаемые действия в очередь, которую выполняет приложение.
//! Зависший плагин прерывается по времени, чтобы не заморозить окно.

mod api;

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

use mlua::{Function, HookTriggers, Lua, Table, VmState};

use crate::context;

// дольше этого один вызов плагина выполняться не должен
const BUDGET: Duration = Duration::from_millis(100);

/// Что плагин может узнать о терминале в момент вызова.
#[derive(Debug, Default, Clone)]
pub struct Snapshot {
    pub title: String,
    pub cwd: String,
    pub selection: String,
    pub screen: String,
}

/// Действие, запрошенное плагином.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Send(String),
    Notify { title: String, body: String },
    Action(String),
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
    _lua: Lua,
}

fn limit_time(lua: &Lua, deadline: Rc<Cell<Instant>>) -> mlua::Result<()> {
    let triggers = HookTriggers {
        every_nth_instruction: Some(10_000),
        ..HookTriggers::default()
    };
    lua.set_global_hook(triggers, move |_, _| {
        if Instant::now() > deadline.get() {
            return Err(mlua::Error::runtime("плагин выполняется слишком долго"));
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

impl Plugins {
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
        let errors = run_scripts(&lua, directory, &deadline);
        Ok((
            Self {
                registry,
                shared,
                deadline,
                _lua: lua,
            },
            errors,
        ))
    }

    fn call(&self, method: &str, arguments: impl mlua::IntoLuaMulti, snapshot: Snapshot) {
        *self.shared.snapshot.borrow_mut() = snapshot;
        self.deadline.set(Instant::now() + BUDGET);
        let result = self
            .registry
            .get::<Function>(method)
            .and_then(|f| f.call::<()>(arguments));
        if let Err(e) = result {
            log::warn!("Плагины: {e}");
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
