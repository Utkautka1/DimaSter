//! Помощь при ошибке: что сделать, если команда упала.
//!
//! Подсказка показывается плашкой окна, а не печатается в терминал: к этому
//! моменту shell уже нарисовал приглашение. Ничего не выполняется само.

use std::time::Duration;

use dimaster_grid::Finished;
use dimaster_i18n::{tr, trf};

use crate::view::View;

// сколько висит плашка: её надо успеть прочитать
const SHOWN: Duration = Duration::from_secs(8);

// сообщения программ о нехватке прав (английские и русские)
const DENIED: [&str; 6] = [
    "permission denied",
    "operation not permitted",
    "are you root",
    "unless you are root",
    "отказано в доступе",
    "операция не позволена",
];

// у этих «permission denied» значит отказ сервера, sudo не поможет
const REMOTE: [&str; 6] = ["sudo", "doas", "ssh", "scp", "sftp", "git"];

// команда поиска пакета: имя пакета не угадываем
const MANAGERS: [(&str, &str); 4] = [
    ("pacman", "pacman -Ss"),
    ("apt", "apt search"),
    ("dnf", "dnf search"),
    ("zypper", "zypper search"),
];

fn program(command: &str) -> &str {
    command.split_whitespace().next().unwrap_or_default()
}

fn says(output: &[String], needles: &[&str]) -> bool {
    output.iter().any(|line| {
        let line = line.to_lowercase();
        needles.iter().any(|n| line.contains(n))
    })
}

/// Совет про права: `chmod` для незапускаемого файла, иначе `sudo`.
fn rights(done: &Finished) -> Option<String> {
    let name = program(&done.command);
    if done.exit == 0 || REMOTE.contains(&name) {
        return None;
    }
    if done.exit == 126 && name.contains('/') {
        return Some(trf(
            "Файл не запускается: chmod +x {file}",
            &[("file", name)],
        ));
    }
    says(&done.output, &DENIED).then(|| tr("Нет прав: Ctrl+Shift+S добавит sudo"))
}

// слово, которое shell не нашёл; присваивания и пути сюда не относятся
fn missing(done: &Finished) -> Option<&str> {
    let name = program(&done.command);
    let plain = !name.is_empty() && !name.contains(['/', '=']);
    // имя в выводе отличает shell от алиаса или скрипта, упавшего внутри
    (done.exit == 127 && plain && says(&done.output, &[&name.to_lowercase()])).then_some(name)
}

fn not_installed(name: &str, search: Option<&str>) -> String {
    let text = trf("Программа {name} не установлена", &[("name", name)]);
    match search {
        Some(search) => format!("{text}\n{search} {name}"),
        None => text,
    }
}

impl View {
    // опечатка важнее установки: `gti` — это `git`, а не новый пакет
    fn missing_advice(&mut self, name: &str) -> Option<String> {
        if self.syntax.exists(name, self.term.cwd()) {
            return None;
        }
        if let Some(fix) = self.syntax.correction(name) {
            return Some(trf("Возможно, вы имели в виду: {fix}", &[("fix", &fix)]));
        }
        let search = MANAGERS
            .iter()
            .find(|m| self.syntax.exists(m.0, None))
            .map(|m| m.1);
        Some(not_installed(name, search))
    }

    /// Показывает совет по только что упавшей команде.
    pub(crate) fn help_with(&mut self, done: &Finished) {
        if !self.config.smart.help {
            return;
        }
        let advice = match missing(done) {
            Some(name) => self.missing_advice(name),
            None => rights(done),
        };
        if let Some(text) = advice {
            self.notice.show(text, SHOWN);
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
        assert!(rights(&done("mkdir /opt/x", 1, denied)).is_some_and(|t| t.contains("sudo")));
        let script = rights(&done("./run.sh", 126, denied));
        assert!(script.is_some_and(|t| t.contains("chmod +x ./run.sh")));
    }

    #[test]
    fn sudo_is_not_offered_where_it_cannot_help() {
        let denied = "Permission denied (publickey).";
        assert_eq!(rights(&done("ssh host", 255, denied)), None);
        assert_eq!(rights(&done("sudo ls /x", 1, denied)), None);
        assert_eq!(rights(&done("cat log", 0, denied)), None);
    }

    #[test]
    fn only_the_typed_program_counts_as_missing() {
        let shell = "zsh: htop: command not found";
        assert_eq!(missing(&done("htop -d 1", 127, shell)), Some("htop"));
        assert_eq!(missing(&done("deploy", 127, "sh: jq: not found")), None);
        assert_eq!(missing(&done("htop", 1, shell)), None);
    }
}
