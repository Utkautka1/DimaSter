//! Разбор списка `keys`.
//!
//! Нажатие пишется либо парой `key` + `mods`, либо одной строкой в `key`:
//! `"ctrl+a>n"`, с приставками `unconsumed:` и `performable:`.

use mlua::{Table, Value};

use crate::{Action, Binding, Chord, Config, ConfigError, Mods};

type Parsed<T> = Result<T, ConfigError>;

// отрезает приставки; `global:` и `all:` честно называются неподдерживаемыми
fn prefixes<'a>(mut text: &'a str, binding: &mut Binding) -> Parsed<&'a str> {
    while let Some((prefix, rest)) = text.split_once(':').filter(|p| p.0.len() > 1) {
        match prefix {
            "unconsumed" => binding.unconsumed = true,
            "performable" => binding.performable = true,
            _ => return Err(ConfigError::new("keys.key", text)),
        }
        text = rest;
    }
    Ok(text)
}

fn chords(entry: &Table, binding: &mut Binding) -> Parsed<Vec<Chord>> {
    let key: String = entry.get("key")?;
    let bad = || ConfigError::new("keys.key", &key);
    let mods: Option<String> = entry.get("mods")?;
    if let Some(mods) = mods {
        let mods = Mods::parse(&mods).ok_or_else(|| ConfigError::new("mods", &mods))?;
        return Ok(vec![Chord::new(&key, mods)]);
    }
    let trigger = prefixes(&key, binding)?;
    // одиночный знак `>` — это клавиша, а не разделитель цепочки
    if trigger.chars().count() == 1 {
        return Ok(vec![Chord::new(trigger, Mods::default())]);
    }
    let parts = trigger.split('>').map(Chord::parse);
    parts.collect::<Option<_>>().ok_or_else(bad)
}

fn action(name: &str) -> Parsed<Action> {
    Action::parse(name).ok_or_else(|| ConfigError::new("action", name))
}

// строка — одно действие, список — несколько подряд
fn actions(value: Value) -> Parsed<Vec<Action>> {
    match value {
        Value::String(name) => Ok(vec![action(&name.to_str()?)?]),
        Value::Table(list) => {
            let names: Vec<String> = list.sequence_values().collect::<mlua::Result<_>>()?;
            names.iter().map(|n| action(n)).collect()
        }
        other => Err(ConfigError::new("action", other.type_name())),
    }
}

fn binding(entry: &Table) -> Parsed<Binding> {
    let mut binding = Binding::new(Vec::new(), actions(entry.get("action")?)?);
    binding.chords = chords(entry, &mut binding)?;
    let flag = |key| entry.get::<Option<bool>>(key);
    binding.unconsumed |= flag("unconsumed")?.unwrap_or(false);
    binding.performable |= flag("performable")?.unwrap_or(false);
    binding.table = entry.get::<Option<String>>("table")?.unwrap_or_default();
    Ok(binding)
}

pub(crate) fn apply(table: &Table, config: &mut Config) -> Parsed<()> {
    if table.get::<Option<bool>>("clear_default_keys")? == Some(true) {
        config.keys.clear();
    }
    let Some(keys) = table.get::<Option<Vec<Table>>>("keys")? else {
        return Ok(());
    };
    for entry in keys {
        config.bind(binding(&entry)?);
    }
    Ok(())
}
