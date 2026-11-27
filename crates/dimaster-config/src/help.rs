//! Таблица `help`: приветствие первого запуска и советы при ошибке.
//!
//! Тексты по умолчанию русские и переводятся при показе; свои показываются как есть.

use mlua::Table;

/// Что и когда подсказывать.
#[derive(Debug, Clone, PartialEq)]
pub struct Help {
    /// Советы плашкой после упавшей команды.
    pub enabled: bool,
    // миллисекунды на экране
    pub duration: u32,
    /// Строки после мастера первого запуска; пусто — без приветствия.
    pub welcome: Vec<String>,
    /// Подсказка исправления опечатки, `{fix}`; пусто — не показывать.
    pub typo: String,
    pub sudo: String,
    // `{file}`
    pub chmod: String,
    // `{name}`
    pub missing: String,
    // `{fix}`
    pub similar: String,
    /// Фразы программ о нехватке прав (в нижнем регистре).
    pub denied: Vec<String>,
    /// Программы, которым `sudo` не поможет.
    pub skip: Vec<String>,
    /// Пары «менеджер пакетов — команда поиска».
    pub managers: Vec<(String, String)>,
}

fn owned(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn welcome() -> Vec<String> {
    owned(&[
        "Добро пожаловать в DimaSter!",
        "  →              принять подсказку команды",
        "  Ctrl+Shift+H   поиск по истории",
        "  Ctrl+Shift+G   переход в частую папку",
        "  Ctrl+Shift+P   все остальные команды",
    ])
}

fn denied() -> Vec<String> {
    owned(&[
        "permission denied",
        "operation not permitted",
        "are you root",
        "unless you are root",
        "отказано в доступе",
        "операция не позволена",
    ])
}

fn managers() -> Vec<(String, String)> {
    [
        ("pacman", "pacman -Ss"),
        ("apt", "apt search"),
        ("dnf", "dnf search"),
        ("zypper", "zypper search"),
    ]
    .map(|(a, b)| (a.to_owned(), b.to_owned()))
    .to_vec()
}

impl Default for Help {
    fn default() -> Self {
        Self {
            enabled: true,
            duration: 8000,
            welcome: welcome(),
            typo: "  ➜ {fix}   Ctrl+→".to_owned(),
            sudo: "Нет прав: Ctrl+Shift+S добавит sudo".to_owned(),
            chmod: "Файл не запускается: chmod +x {file}".to_owned(),
            missing: "Программа {name} не установлена".to_owned(),
            similar: "Возможно, вы имели в виду: {fix}".to_owned(),
            denied: denied(),
            skip: owned(&["sudo", "doas", "ssh", "scp", "sftp", "git"]),
            managers: managers(),
        }
    }
}

fn set<T: mlua::FromLua>(table: &Table, key: &str, target: &mut T) -> mlua::Result<()> {
    if let Some(value) = table.get::<Option<T>>(key)? {
        *target = value;
    }
    Ok(())
}

fn texts(table: &Table, help: &mut Help) -> mlua::Result<()> {
    set(table, "typo", &mut help.typo)?;
    set(table, "sudo", &mut help.sudo)?;
    set(table, "chmod", &mut help.chmod)?;
    set(table, "missing", &mut help.missing)?;
    set(table, "similar", &mut help.similar)
}

// пара записывается как { "apt", "apt search" }
fn pairs(table: &Table, help: &mut Help) -> mlua::Result<()> {
    let Some(rows) = table.get::<Option<Vec<Vec<String>>>>("managers")? else {
        return Ok(());
    };
    help.managers = rows
        .into_iter()
        .filter_map(|row| <[String; 2]>::try_from(row).ok())
        .map(|[name, search]| (name, search))
        .collect();
    Ok(())
}

pub(crate) fn apply(table: &Table, help: &mut Help) -> mlua::Result<()> {
    set(table, "enabled", &mut help.enabled)?;
    set(table, "duration", &mut help.duration)?;
    help.duration = help.duration.clamp(200, 60_000);
    set(table, "welcome", &mut help.welcome)?;
    set(table, "denied", &mut help.denied)?;
    set(table, "skip", &mut help.skip)?;
    texts(table, help)?;
    pairs(table, help)
}
