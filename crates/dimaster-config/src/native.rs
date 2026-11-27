//! Таблица `native`: удобства shell силами самого терминала.

use mlua::Table;

use crate::tuning::chords;
use crate::{Chord, ConfigError};

/// Поиск по истории, сокращения, алиасы и меню дополнения.
#[derive(Debug, Clone, PartialEq)]
pub struct Native {
    /// Стрелки листают команды, начинающиеся с набранного.
    pub history_search: bool,
    pub history_back: Vec<Chord>,
    pub history_forward: Vec<Chord>,
    /// Меню дополнения; чего оно не знает, достаётся shell.
    pub complete: bool,
    pub complete_keys: Vec<Chord>,
    /// Команды, аргументы которых дополняются файлами.
    pub path_commands: Vec<String>,
    /// Брать флаги из `man`.
    pub man_flags: bool,
    /// Разворачиваются пробелом после первого слова.
    pub abbreviations: Vec<(String, String)>,
    /// Алиасы shell, одинаковые для zsh, bash и fish.
    pub aliases: Vec<(String, String)>,
}

impl Default for Native {
    fn default() -> Self {
        let commands = "cd ls ll cat bat less more head tail vim nvim nano micro code cp mv rm \
                        mkdir rmdir touch source open du stat file chmod chown";
        Self {
            history_search: true,
            history_back: chords(&["up"]),
            history_forward: chords(&["down"]),
            complete: true,
            complete_keys: chords(&["tab"]),
            path_commands: commands.split_whitespace().map(str::to_owned).collect(),
            man_flags: true,
            abbreviations: Vec::new(),
            aliases: Vec::new(),
        }
    }
}

fn set<T: mlua::FromLua>(table: &Table, key: &str, target: &mut T) -> mlua::Result<()> {
    if let Some(value) = table.get::<Option<T>>(key)? {
        *target = value;
    }
    Ok(())
}

fn keys(table: &Table, key: &str, target: &mut Vec<Chord>) -> mlua::Result<()> {
    if let Some(texts) = table.get::<Option<Vec<String>>>(key)? {
        let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
        *target = chords(&texts);
    }
    Ok(())
}

// имя попадает в текст для shell, поэтому только безопасные знаки
fn valid(name: &str, value: &str) -> bool {
    let plain = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    !name.is_empty()
        && !name.starts_with('-')
        && name.chars().all(plain)
        && !value.chars().any(char::is_control)
}

fn words(table: &Table, key: &str, target: &mut Vec<(String, String)>) -> Result<(), ConfigError> {
    let Some(found) = table.get::<Option<Table>>(key)? else {
        return Ok(());
    };
    for pair in found.pairs::<String, String>() {
        let (name, value) = pair?;
        if !valid(&name, &value) {
            return Err(ConfigError::new(key, &name));
        }
        target.retain(|t| t.0 != name);
        target.push((name, value));
    }
    // порядок таблицы Lua случаен
    target.sort();
    Ok(())
}

fn switches(table: &Table, native: &mut Native) -> mlua::Result<()> {
    set(table, "history_search", &mut native.history_search)?;
    set(table, "complete", &mut native.complete)?;
    set(table, "man_flags", &mut native.man_flags)?;
    set(table, "path_commands", &mut native.path_commands)?;
    keys(table, "history_back", &mut native.history_back)?;
    keys(table, "history_forward", &mut native.history_forward)?;
    keys(table, "complete_keys", &mut native.complete_keys)
}

pub(crate) fn apply(table: &Table, native: &mut Native) -> Result<(), ConfigError> {
    switches(table, native)?;
    words(table, "abbreviations", &mut native.abbreviations)?;
    words(table, "aliases", &mut native.aliases)
}
