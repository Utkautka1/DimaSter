//! Фоновые потоки: списки, статистика и действия.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread;
use std::time::Duration;

use serde_json::Value;

use crate::host::Host;
use crate::kind::{Item, Kind};
use crate::process::capture;

const QUERY: Duration = Duration::from_secs(10);
// остановка контейнера ждёт его завершения, образы удаляются долго
const ACTION: Duration = Duration::from_secs(120);

/// Что нужно опрашивать сейчас.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) host: Host,
    pub(crate) kind: Kind,
}

pub(crate) type Stats = HashMap<String, (String, String)>;

pub(crate) enum Event {
    Listed(Target, Result<Vec<Item>, String>),
    Stats(Host, Stats),
    Done(Result<(), String>),
    Shown(Result<String, String>),
}

// CPU и память по коротким номерам контейнеров
fn parse_stats(output: &str) -> Stats {
    let records = output
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok());
    let text = |r: &Value, key: &str| r[key].as_str().unwrap_or_default().to_owned();
    records
        .map(|r| (text(&r, "ID"), (text(&r, "CPUPerc"), text(&r, "MemUsage"))))
        .collect()
}

fn list(target: &Target) -> Result<Vec<Item>, String> {
    target.host.connect();
    let command = target.host.command(&target.kind.list_args());
    capture(command, QUERY).map(|text| target.kind.parse(&text))
}

fn stats(host: &Host) -> Option<Stats> {
    host.connect();
    let args = ["stats", "--no-stream", "--format", "{{json .}}"].map(String::from);
    let text = capture(host.command(&args), QUERY).ok()?;
    Some(parse_stats(&text))
}

// опрашивает по таймеру и сразу при смене цели; умирает вместе с приложением
fn repeat<T: Send + Clone + PartialEq + 'static>(
    period: Duration,
    mut work: impl FnMut(&T) -> bool + Send + 'static,
) -> Sender<T> {
    let (send, targets) = channel::<T>();
    thread::spawn(move || {
        let Ok(mut target) = targets.recv() else {
            return;
        };
        loop {
            target = targets.try_iter().last().unwrap_or(target);
            if !work(&target) {
                return;
            }
            match targets.recv_timeout(period) {
                Ok(next) => target = next,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    });
    send
}

/// Связь с фоновыми потоками.
pub(crate) struct Worker {
    lists: Sender<Target>,
    stats: Option<Sender<Target>>,
    events: Sender<Event>,
    pub(crate) incoming: Receiver<Event>,
}

impl Worker {
    pub(crate) fn start(period: Duration, with_stats: bool) -> Self {
        let (events, incoming) = channel();
        let out = events.clone();
        let lists = repeat(period, move |t: &Target| {
            out.send(Event::Listed(t.clone(), list(t))).is_ok()
        });
        let out = events.clone();
        let stats = with_stats.then(|| {
            repeat(period, move |t: &Target| {
                let found = (t.kind == Kind::Containers)
                    .then(|| stats(&t.host))
                    .flatten();
                found.is_none_or(|s| out.send(Event::Stats(t.host.clone(), s)).is_ok())
            })
        });
        Self {
            lists,
            stats,
            events,
            incoming,
        }
    }

    /// Новая цель или просьба обновить немедленно.
    pub(crate) fn watch(&self, target: &Target) {
        // поток мог уже завершиться вместе с программой
        let _ = self.lists.send(target.clone());
        if let Some(stats) = &self.stats {
            let _ = stats.send(target.clone());
        }
    }

    /// Выполняет команду; `show` — вернуть её вывод.
    pub(crate) fn run(&self, host: &Host, args: &[String], show: bool) {
        let (host, args) = (host.clone(), args.to_vec());
        let events = self.events.clone();
        thread::spawn(move || {
            host.connect();
            let result = capture(host.command(&args), ACTION);
            let event = match show {
                true => Event::Shown(result),
                false => Event::Done(result.map(drop)),
            };
            let _ = events.send(event);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_are_keyed_by_container() {
        let found = parse_stats(r#"{"ID":"abc","CPUPerc":"1%","MemUsage":"2MiB / 1GiB"}"#);
        assert_eq!(found["abc"].0, "1%");
    }
}
