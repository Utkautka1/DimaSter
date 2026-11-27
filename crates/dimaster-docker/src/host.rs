//! Где работает Docker: здесь или на сервере из `~/.ssh/config`.

use std::process::Command;

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

    /// `docker` с этими аргументами на этой машине.
    pub(crate) fn command(&self, args: &[String]) -> Command {
        let mut command = Command::new("docker");
        if let Self::Ssh(name) = self {
            command.args(["-H", &format!("ssh://{name}")]);
            // ssh спросил бы пароль прямо поверх экрана: пусть вместо этого откажет
            command
                .env("SSH_ASKPASS", "false")
                .env("SSH_ASKPASS_REQUIRE", "force");
        }
        command.args(args);
        command
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
