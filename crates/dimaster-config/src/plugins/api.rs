//! Функции таблицы `dimaster`, реализованные на стороне Rust.

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

fn effects(lua: &Lua, api: &Table, shared: &Rc<Shared>) -> mlua::Result<()> {
    let queue = shared.clone();
    api.set(
        "send",
        lua.create_function(move |_, text: String| {
            push(&queue, Effect::Send(text));
            Ok(())
        })?,
    )?;
    let queue = shared.clone();
    api.set(
        "action",
        lua.create_function(move |_, name: String| {
            push(&queue, Effect::Action(name));
            Ok(())
        })?,
    )?;
    let queue = shared.clone();
    api.set(
        "notify",
        lua.create_function(move |_, (title, body): (String, String)| {
            push(&queue, Effect::Notify { title, body });
            Ok(())
        })?,
    )
}

// какое поле снимка отдаёт функция
type Field = fn(&Snapshot) -> &String;

fn queries(lua: &Lua, api: &Table, shared: &Rc<Shared>) -> mlua::Result<()> {
    let fields: [(&str, Field); 4] = [
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
        log::warn!("плагин: {message}");
        Ok(())
    })?;
    api.set("log", log)
}
