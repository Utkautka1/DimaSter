//! Какими аргументами `docker` выполняется действие над строкой вкладки.

use crate::host::Host;
use crate::kind::{Item, Kind};

/// Что сделать после нажатия.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Task {
    /// Выполнить и обновить список.
    Run(Vec<String>),
    /// Показать вывод.
    Show(Vec<String>),
    /// Следить за выводом.
    Follow(Vec<String>),
    /// Занять терминал программой.
    Attach(Vec<String>),
}

fn words(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|p| (*p).to_owned()).collect()
}

fn running(item: &Item) -> bool {
    item.cells
        .get(1)
        .is_some_and(|s| s == "running" || s == "paused")
}

fn container(action: &str, item: &Item, tail: &str, shell: &[String]) -> Option<Task> {
    let id = item.id.as_str();
    let run = |verb: &str| Some(Task::Run(words(&[verb, id])));
    match action {
        "toggle" if running(item) => run("stop"),
        "toggle" => run("start"),
        "restart" => run("restart"),
        "pause" if item.cells.get(1).is_some_and(|s| s == "paused") => run("unpause"),
        "pause" => run("pause"),
        "remove" => run("rm"),
        "logs" => Some(Task::Follow(words(&[
            "logs", "--follow", "--tail", tail, id,
        ]))),
        "shell" => Some(Task::Attach(
            [words(&["exec", "-it", id]), shell.to_vec()].concat(),
        )),
        "prune" => Some(Task::Run(words(&["container", "prune", "--force"]))),
        _ => None,
    }
}

fn compose(action: &str, item: &Item, tail: &str, host: &Host) -> Option<Task> {
    let project = words(&["compose", "--project-name", &item.id]);
    let with = |rest: &[&str]| [project.clone(), words(rest)].concat();
    match action {
        "toggle" if item.cells.get(1).is_some_and(|s| s.contains("running")) => {
            Some(Task::Run(with(&["stop"])))
        }
        "toggle" => Some(Task::Run(with(&["start"]))),
        "restart" => Some(Task::Run(with(&["restart"]))),
        "down" | "remove" => Some(Task::Run(with(&["down"]))),
        "logs" => Some(Task::Follow(with(&["logs", "--follow", "--tail", tail]))),
        // файлы проекта лежат на той же машине, что и демон: по SSH их не прочитать
        "up" if *host == Host::Local && !item.files.is_empty() => {
            let files = item
                .files
                .iter()
                .flat_map(|f| ["--file".to_owned(), f.clone()]);
            Some(Task::Run(
                [project.clone(), files.collect(), words(&["up", "--detach"])].concat(),
            ))
        }
        _ => None,
    }
}

// образы, тома и сети: удалить, очистить, посмотреть
fn resource(kind: &str, action: &str, item: &Item) -> Option<Task> {
    match action {
        "remove" => Some(Task::Run(words(&[kind, "rm", &item.id]))),
        "prune" => Some(Task::Run(words(&[kind, "prune", "--force"]))),
        _ => None,
    }
}

/// Откуда берутся параметры действий.
pub(crate) struct Context<'a> {
    pub(crate) host: &'a Host,
    pub(crate) tail: usize,
    pub(crate) shell: &'a [String],
}

/// `None` — на этой вкладке действие не имеет смысла.
pub(crate) fn task(kind: Kind, action: &str, item: &Item, context: &Context<'_>) -> Option<Task> {
    if action == "inspect" && kind != Kind::Compose {
        return Some(Task::Show(words(&["inspect", &item.id])));
    }
    let tail = context.tail.to_string();
    match kind {
        Kind::Containers => container(action, item, &tail, context.shell),
        Kind::Compose => compose(action, item, &tail, context.host),
        Kind::Images => resource("image", action, item),
        Kind::Volumes => resource("volume", action, item),
        Kind::Networks => resource("network", action, item),
    }
}

/// Действия, которые что-то удаляют.
pub(crate) fn destructive(action: &str) -> bool {
    matches!(action, "remove" | "prune" | "down")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(state: &str) -> Item {
        Item {
            id: "abc".into(),
            cells: vec!["web".into(), state.into()],
            ..Item::default()
        }
    }

    fn found(kind: Kind, action: &str, state: &str) -> Option<Task> {
        let context = Context {
            host: &Host::Local,
            tail: 10,
            shell: &["sh".into()],
        };
        task(kind, action, &item(state), &context)
    }

    #[test]
    fn toggle_follows_state() {
        let run = |a: &[&str]| Some(Task::Run(words(a)));
        assert_eq!(
            found(Kind::Containers, "toggle", "running"),
            run(&["stop", "abc"])
        );
        assert_eq!(
            found(Kind::Containers, "toggle", "exited"),
            run(&["start", "abc"])
        );
        assert_eq!(
            found(Kind::Volumes, "remove", ""),
            run(&["volume", "rm", "abc"])
        );
        assert_eq!(found(Kind::Images, "toggle", ""), None);
        assert_eq!(found(Kind::Compose, "up", ""), None);
    }
}
