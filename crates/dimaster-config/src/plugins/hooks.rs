//! Клавиши, таймеры и событие команды: то, что плагин делает по ходу работы.

use mlua::Function;

use super::{Effect, Plugins, Snapshot};
use crate::Chord;

impl Plugins {
    /// Нажатия, привязанные через `dimaster.bind`, в порядке регистрации.
    pub fn keys(&self) -> Vec<Chord> {
        let list = self
            .registry
            .get::<Function>("keys")
            .and_then(|f| f.call::<Vec<String>>(()));
        // неразобранная строка не должна сдвигать номера остальных
        let parse = |text: &String| {
            Chord::parse(text).unwrap_or_else(|| Chord::new("", Default::default()))
        };
        list.unwrap_or_default().iter().map(parse).collect()
    }

    /// Нажатие номер `index`; `false` — обработчик отдал клавишу дальше.
    pub fn key(&self, index: usize, snapshot: Snapshot) -> (bool, Vec<Effect>) {
        self.prepare_call(snapshot);
        let consumed = self
            .registry
            .get::<Function>("key")
            .and_then(|f| f.call::<bool>(index + 1));
        let consumed = consumed.unwrap_or_else(|e| {
            log::warn!("bind: {e}");
            false
        });
        (consumed, self.shared.effects.take())
    }

    /// Периоды таймеров `dimaster.every` в секундах.
    pub fn timers(&self) -> Vec<f64> {
        let list = self
            .registry
            .get::<Function>("intervals")
            .and_then(|f| f.call::<Vec<f64>>(()));
        list.unwrap_or_default()
    }

    pub fn tick(&self, index: usize, snapshot: Snapshot) -> Vec<Effect> {
        self.call("tick", index + 1, snapshot);
        self.shared.effects.take()
    }

    /// Событие `command`: текст, код завершения и длительность в секундах.
    pub fn command_finished(
        &self,
        command: &str,
        exit: i32,
        seconds: f64,
        snapshot: Snapshot,
    ) -> Vec<Effect> {
        self.call("emit", ("command", command, exit, seconds), snapshot);
        self.shared.effects.take()
    }
}
