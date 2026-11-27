//! Ключ `snippets`: свои команды с полями `{{имя}}`.

use mlua::Table;

use crate::{Config, ConfigError};

/// Команда из списка сниппетов.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    pub name: String,
    pub command: String,
}

fn snippet(entry: &Table) -> Result<Snippet, ConfigError> {
    let command: String = entry.get("command")?;
    // перевод строки выполнил бы команду при вставке
    if command.chars().any(char::is_control) {
        return Err(ConfigError::new(
            "snippets.command",
            "одна строка без управляющих символов",
        ));
    }
    let name = entry.get::<Option<String>>("name")?;
    Ok(Snippet {
        name: name.unwrap_or_else(|| command.clone()),
        command,
    })
}

pub(crate) fn apply(table: &Table, config: &mut Config) -> Result<(), ConfigError> {
    if let Some(list) = table.get::<Option<Vec<Table>>>("snippets")? {
        config.snippets = list.iter().map(snippet).collect::<Result<_, _>>()?;
    }
    Ok(())
}
