//! Помощь при ошибке: что сделать, если команда упала.
//!
//! Подсказка показывается плашкой окна, а не печатается в терминал: к этому
//! моменту shell уже нарисовал приглашение. Ничего не выполняется само.

use std::time::Duration;

use dimaster_config::Help;
use dimaster_grid::Finished;
use dimaster_i18n::tr;

use crate::view::View;

fn program(command: &str) -> &str {
    command.split_whitespace().next().unwrap_or_default()
}

fn says(output: &[String], needles: &[String]) -> bool {
    output.iter().any(|line| {
        let line = line.to_lowercase();
        needles.iter().any(|n| line.contains(&n.to_lowercase()))
    })
}

// свой текст из Lua остаётся как есть, стандартный переводится
fn text(template: &str, key: &str, value: &str) -> String {
    tr(template).replace(key, value)
}

/// Совет про права: `chmod` для незапускаемого файла, иначе `sudo`.
fn rights(done: &Finished, help: &Help) -> Option<String> {
    let name = program(&done.command);
    if done.exit == 0 || help.skip.iter().any(|s| s == name) {
        return None;
    }
    if done.exit == 126 && name.contains('/') {
        return Some(text(&help.chmod, "{file}", name));
    }
    says(&done.output, &help.denied).then(|| tr(&help.sudo))
}

// слово, которое shell не нашёл; присваивания и пути сюда не относятся
fn missing(done: &Finished) -> Option<&str> {
    let name = program(&done.command);
    let plain = !name.is_empty() && !name.contains(['/', '=']);
    // имя в выводе отличает shell от алиаса или скрипта, упавшего внутри
    let named = says(&done.output, &[name.to_owned()]);
    (done.exit == 127 && plain && named).then_some(name)
}

fn not_installed(template: &str, name: &str, search: Option<&str>) -> String {
    let message = text(template, "{name}", name);
    match search {
        Some(search) => format!("{message}\n{search} {name}"),
        None => message,
    }
}

impl View {
    // опечатка важнее установки: `gti` — это `git`, а не новый пакет
    fn missing_advice(&mut self, name: &str) -> Option<String> {
        if self.syntax.exists(name, self.term.cwd()) {
            return None;
        }
        let help = &self.config.help;
        if let Some(fix) = self.syntax.correction(name) {
            return Some(text(&help.similar, "{fix}", &fix));
        }
        let search = help
            .managers
            .iter()
            .find(|m| self.syntax.exists(&m.0, None))
            .map(|m| m.1.as_str());
        Some(not_installed(&help.missing, name, search))
    }

    /// Показывает совет по только что упавшей команде.
    pub(crate) fn help_with(&mut self, done: &Finished) {
        if !self.config.help.enabled {
            return;
        }
        let advice = match missing(done) {
            Some(name) => self.missing_advice(name),
            None => rights(done, &self.config.help),
        };
        if let Some(advice) = advice.filter(|a| !a.is_empty()) {
            let shown = Duration::from_millis(u64::from(self.config.help.duration));
            self.notice.show(advice, shown);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn done(command: &str, exit: i32, output: &str) -> Finished {
        Finished {
            command: command.to_owned(),
            exit,
            duration: Duration::ZERO,
            output: vec![output.to_owned()],
        }
    }

    #[test]
    fn denied_command_gets_sudo_or_chmod() {
        let denied = "mkdir: /opt/x: Permission denied";
        assert!(
            rights(&done("mkdir /opt/x", 1, denied), &Help::default())
                .is_some_and(|t| t.contains("sudo"))
        );
        let script = rights(&done("./run.sh", 126, denied), &Help::default());
        assert!(script.is_some_and(|t| t.contains("chmod +x ./run.sh")));
    }

    #[test]
    fn sudo_is_not_offered_where_it_cannot_help() {
        let denied = "Permission denied (publickey).";
        assert_eq!(
            rights(&done("ssh host", 255, denied), &Help::default()),
            None
        );
        assert_eq!(
            rights(&done("sudo ls /x", 1, denied), &Help::default()),
            None
        );
        assert_eq!(rights(&done("cat log", 0, denied), &Help::default()), None);
    }

    #[test]
    fn only_the_typed_program_counts_as_missing() {
        let shell = "zsh: htop: command not found";
        assert_eq!(missing(&done("htop -d 1", 127, shell)), Some("htop"));
        assert_eq!(missing(&done("deploy", 127, "sh: jq: not found")), None);
        assert_eq!(missing(&done("htop", 1, shell)), None);
    }
}
