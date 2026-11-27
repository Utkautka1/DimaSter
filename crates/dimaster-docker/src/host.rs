//! Где работает Docker: здесь или на сервере из `~/.ssh/config`.

use std::process::Command;

use crate::{mux, tunnel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Host {
    Local,
    Ssh(String),
}

impl Host {
    /// Сервер по имени; `None` — имя похоже на ключ или содержит лишнее.
    pub fn ssh(name: &str) -> Option<Self> {
        let allowed = |c: char| c.is_ascii_alphanumeric() || "._-@:".contains(c);
        let safe = !name.is_empty() && !name.starts_with('-') && name.chars().all(allowed);
        safe.then(|| Self::Ssh(name.to_owned()))
    }

    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Local => "local",
            Self::Ssh(name) => name,
        }
    }

    /// Долгое подключение к серверу; вызывается из фоновых потоков.
    pub(crate) fn connect(&self) {
        if let Self::Ssh(name) = self {
            tunnel::ensure(name);
        }
    }

    /// `docker` с этими аргументами на этой машине.
    pub(crate) fn command(&self, args: &[String]) -> Command {
        let mut command = Command::new("docker");
        if let Self::Ssh(name) = self {
            match tunnel::socket(name) {
                Some(socket) => {
                    command
                        .arg("-H")
                        .arg(format!("unix://{}", socket.display()));
                }
                None => over_ssh(&mut command, name),
            }
        }
        command.args(args);
        command
    }
}

// запасной путь без туннеля: клиент `docker` сам запускает `ssh`
fn over_ssh(command: &mut Command, name: &str) {
    command.args(["-H", &format!("ssh://{name}")]);
    // ssh спросил бы пароль прямо поверх экрана: пусть вместо этого откажет
    command
        .env("SSH_ASKPASS", "false")
        .env("SSH_ASKPASS_REQUIRE", "force");
    // соединение переиспользуется и здесь
    if let Some(path) = mux::path() {
        command.env("PATH", path);
    }
}

/// Для команды, занимающей терминал: ssh снова может спрашивать пароль.
pub(crate) fn allow_prompts(command: &mut Command) {
    command
        .env_remove("SSH_ASKPASS_REQUIRE")
        .env_remove("SSH_ASKPASS");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_goes_through_ssh_url() {
        let host = Host::ssh("web").expect("имя");
        let command = host.command(&["ps".into()]);
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, ["-H", "ssh://web", "ps"]);
        assert!(Host::ssh("-oProxyCommand=x").is_none());
        assert!(Host::ssh("a b").is_none());
    }
}
