//! Таблица `limits`: значения сами по себе не действуют, их применяет окно.

use dimaster_limits::ALL;
use mlua::Table;

// опечатка в имени не должна молча оставить прежний предел
fn known(name: &str) -> mlua::Result<usize> {
    let found = ALL.iter().position(|l| l.name == name);
    found.ok_or_else(|| mlua::Error::runtime(format!("limits.{name}: ?")))
}

/// Добавляет выбранные пределы; повторное имя заменяет прежнее значение.
pub(crate) fn apply(table: &Table, chosen: &mut Vec<(String, u64)>) -> mlua::Result<()> {
    for pair in table.pairs::<String, u64>() {
        let (name, value) = pair?;
        known(&name)?;
        chosen.retain(|c| c.0 != name);
        chosen.push((name, value));
    }
    // порядок таблицы Lua случаен
    chosen.sort_by_key(|c| known(&c.0).unwrap_or(usize::MAX));
    Ok(())
}
