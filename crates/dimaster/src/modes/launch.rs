//! Что сделать с выбранным в списках SSH, сниппетов и Docker.

use super::Press;

/// Команда для shell: `run` — выполнить, иначе только вставить.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Command {
    pub(crate) text: String,
    pub(crate) run: bool,
}

/// Подключение по Enter; Shift+Enter оставляет команду в строке.
pub(crate) fn ssh(host: &str, shift: bool) -> Command {
    Command {
        text: format!("ssh {host}"),
        run: !shift,
    }
}

/// Действие над контейнером по клавише; `None` — клавиша не про это.
pub(crate) fn docker(name: &str, up: bool, press: Press) -> Option<Command> {
    let (text, run) = match press {
        Press::Enter { shift } => match up {
            true => (format!("docker exec -it {name} sh"), !shift),
            false => (format!("docker start {name}"), !shift),
        },
        Press::Ctrl('l') => (format!("docker logs -f --tail 200 {name}"), true),
        Press::Ctrl('s') => {
            let verb = if up { "stop" } else { "start" };
            (format!("docker {verb} {name}"), true)
        }
        Press::Ctrl('r') => (format!("docker restart {name}"), true),
        _ => return None,
    };
    Some(Command { text, run })
}

// `{{имя}}`: одиночные скобки оставлены shell и awk
fn placeholder(template: &str) -> Option<(usize, usize, &str)> {
    let start = template.find("{{")?;
    let length = template[start + 2..].find("}}")?;
    let name = &template[start + 2..start + 2 + length];
    let plain = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || "_- ".contains(c));
    plain.then_some((start, start + length + 4, name))
}

/// Имя первого незаполненного поля.
pub(crate) fn next_field(template: &str) -> Option<&str> {
    placeholder(template).map(|p| p.2)
}

/// Подставляет значение во все поля с именем первого.
pub(crate) fn fill_field(template: &str, value: &str) -> String {
    match placeholder(template) {
        Some((start, end, _)) => template.replace(&template[start..end], value),
        None => template.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_keys_depend_on_state() {
        let enter = Press::Enter { shift: false };
        assert_eq!(
            docker("a", true, enter).unwrap().text,
            "docker exec -it a sh"
        );
        assert_eq!(docker("a", false, enter).unwrap().text, "docker start a");
        assert_eq!(
            docker("a", true, Press::Ctrl('s')).unwrap().text,
            "docker stop a"
        );
        assert!(docker("a", true, Press::Char('x')).is_none());
        assert!(!ssh("h", true).run);
    }

    #[test]
    fn only_double_braces_are_fields() {
        let template = "awk '{print $1}' {{file}} > {{file}}.{{ext}}";
        assert_eq!(next_field(template), Some("file"));
        let once = fill_field(template, "a");
        assert_eq!(once, "awk '{print $1}' a > a.{{ext}}");
        assert_eq!(next_field(&once), Some("ext"));
        assert_eq!(next_field("docker ps --format '{{.Names}}'"), None);
    }
}
