//! Настройки `dimaster docker` из `docker.lua`.
//!
//! Колонки, цвета, клавиши, функции `format` и `status` устроены так же, как в
//! `view.lua`, поэтому разбираются тем же кодом; здесь только своё.

use mlua::Table;

use crate::{ConfigError, Viewer, config_dir};

/// Своя команда `docker` по клавише; `{id}` и `{name}` подставляются.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerCommand {
    pub key: String,
    /// Вкладки, где работает; пусто — на всех.
    pub tabs: Vec<String>,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerSettings {
    pub enabled: bool,
    /// Период обновления в секундах.
    pub refresh: u64,
    /// Спрашивать перед удалением.
    pub confirm: bool,
    pub stats: bool,
    pub log_tail: usize,
    pub shell: Vec<String>,
    /// Серверы; `None` — все из `~/.ssh/config`.
    pub hosts: Option<Vec<String>>,
    pub commands: Vec<DockerCommand>,
    pub timing: DockerTiming,
}

/// Ожидания и повторы, все в секундах.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockerTiming {
    pub query_timeout: u64,
    pub action_timeout: u64,
    pub ssh_persist: u64,
    pub tunnel_timeout: u64,
    pub tunnel_retry: u64,
}

impl Default for DockerTiming {
    fn default() -> Self {
        Self {
            query_timeout: 10,
            action_timeout: 120,
            ssh_persist: 60,
            tunnel_timeout: 10,
            tunnel_retry: 60,
        }
    }
}

impl Default for DockerSettings {
    fn default() -> Self {
        let shell = "command -v bash >/dev/null && exec bash || exec sh";
        Self {
            enabled: true,
            refresh: 3,
            confirm: true,
            stats: true,
            log_tail: 500,
            shell: ["sh", "-c", shell].map(String::from).to_vec(),
            hosts: None,
            commands: Vec::new(),
            timing: DockerTiming::default(),
        }
    }
}

/// Загруженный `docker.lua`.
#[derive(Default)]
pub struct Docker {
    pub settings: DockerSettings,
    pub view: Viewer,
}

fn command(table: &Table) -> mlua::Result<DockerCommand> {
    Ok(DockerCommand {
        key: table.get("key")?,
        tabs: table
            .get::<Option<Vec<String>>>("tabs")?
            .unwrap_or_default(),
        args: table.get("args")?,
    })
}

fn commands(root: &Table) -> mlua::Result<Vec<DockerCommand>> {
    let listed = root.get::<Option<Vec<Table>>>("commands")?;
    listed.unwrap_or_default().iter().map(command).collect()
}

fn timing(root: &Table) -> mlua::Result<DockerTiming> {
    let base = DockerTiming::default();
    let seconds = |key: &str, default: u64| {
        let found = root.get::<Option<u64>>(key)?;
        Ok::<_, mlua::Error>(found.unwrap_or(default).clamp(1, 3600))
    };
    Ok(DockerTiming {
        query_timeout: seconds("query_timeout", base.query_timeout)?,
        action_timeout: seconds("action_timeout", base.action_timeout)?,
        ssh_persist: seconds("ssh_persist", base.ssh_persist)?,
        tunnel_timeout: seconds("tunnel_timeout", base.tunnel_timeout)?,
        tunnel_retry: seconds("tunnel_retry", base.tunnel_retry)?,
    })
}

fn settings(root: &Table) -> mlua::Result<DockerSettings> {
    let base = DockerSettings::default();
    let flag = |key: &str, default: bool| {
        Ok::<_, mlua::Error>(root.get::<Option<bool>>(key)?.unwrap_or(default))
    };
    Ok(DockerSettings {
        enabled: flag("enabled", base.enabled)?,
        refresh: root
            .get::<Option<u64>>("refresh")?
            .unwrap_or(base.refresh)
            .max(1),
        confirm: flag("confirm", base.confirm)?,
        stats: flag("stats", base.stats)?,
        log_tail: root
            .get::<Option<usize>>("log_tail")?
            .unwrap_or(base.log_tail),
        shell: root
            .get::<Option<Vec<String>>>("shell")?
            .unwrap_or(base.shell),
        hosts: root.get("hosts")?,
        commands: commands(root)?,
        timing: timing(root)?,
    })
}

impl Docker {
    /// `docker.lua` из каталога настроек; без файла — значения по умолчанию.
    pub fn load() -> Result<Self, ConfigError> {
        let path = config_dir().map(|d| d.join("docker.lua"));
        let Some(path) = path.filter(|p| p.is_file()) else {
            return Ok(Self::default());
        };
        let source = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::new("docker.lua", &e.to_string()))?;
        Self::parse(&source)
    }

    /// Разбирает текст `docker.lua`.
    pub fn parse(source: &str) -> Result<Self, ConfigError> {
        let view = Viewer::parse(source)?;
        let settings = match view.root() {
            Some(root) => settings(root)?,
            None => DockerSettings::default(),
        };
        Ok(Self { settings, view })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_keys_and_shared_view_rules() {
        let source = "return { refresh = 0, hosts = { 'web' }, gap = 3, \
            commands = { { key = 'v', args = { 'top', '{id}' } } } }";
        let docker = Docker::parse(source).expect("docker.lua");
        assert_eq!((docker.settings.refresh, docker.view.settings.gap), (1, 3));
        assert_eq!(docker.settings.commands[0].args, ["top", "{id}"]);
        assert!(Docker::parse("return { commands = { {} } }").is_err());
    }
}
