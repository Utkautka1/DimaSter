//! Функции таблицы `dimaster`, реализованные на стороне Rust.

use dimaster_i18n::t;
use std::rc::Rc;

use mlua::{Lua, Table};

use super::{Effect, Shared, Snapshot};

// очередь ограничена, чтобы цикл в плагине не съел память
const MAX_EFFECTS: usize = 256;

fn push(shared: &Shared, effect: Effect) {
    let mut effects = shared.effects.borrow_mut();
    if effects.len() < MAX_EFFECTS {
        effects.push(effect);
    }
}

// функция Lua, которая лишь ставит действие в очередь
fn queue<A: mlua::FromLuaMulti + 'static>(
    lua: &Lua,
    api: &Table,
    shared: &Rc<Shared>,
    name: &str,
    effect: impl Fn(A) -> Effect + 'static,
) -> mlua::Result<()> {
    let shared = shared.clone();
    let function = move |_: &Lua, arguments: A| {
        push(&shared, effect(arguments));
        Ok(())
    };
    api.set(name, lua.create_function(function)?)
}

fn effects(lua: &Lua, api: &Table, shared: &Rc<Shared>) -> mlua::Result<()> {
    queue(lua, api, shared, "send", Effect::Send)?;
    queue(lua, api, shared, "action", Effect::Action)?;
    queue(lua, api, shared, "set_literal", Effect::Set)?;
    queue(lua, api, shared, "reset", |(): ()| Effect::Reset)?;
    queue(lua, api, shared, "notify", |(title, body)| Effect::Notify {
        title,
        body,
    })
}

// какое поле снимка отдаёт функция
type Field = fn(&Snapshot) -> &String;

fn queries(lua: &Lua, api: &Table, shared: &Rc<Shared>) -> mlua::Result<()> {
    let fields: [(&str, Field); 5] = [
        ("history", |s| &s.history),
        ("title", |s| &s.title),
        ("cwd", |s| &s.cwd),
        ("selection", |s| &s.selection),
        ("screen_text", |s| &s.screen),
    ];
    for (name, field) in fields {
        let source = shared.clone();
        let read = move |_: &Lua, (): ()| Ok(field(&source.snapshot.borrow()).clone());
        api.set(name, lua.create_function(read)?)?;
    }
    Ok(())
}

pub(super) fn install(lua: &Lua, shared: &Rc<Shared>) -> mlua::Result<()> {
    let api: Table = lua.globals().get("dimaster")?;
    effects(lua, &api, shared)?;
    queries(lua, &api, shared)?;
    let log = lua.create_function(|_, message: String| {
        log::warn!("{}", t!("плагин: {message}", message = message));
        Ok(())
    })?;
    api.set("log", log)
}
