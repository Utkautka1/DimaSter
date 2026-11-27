//! То, что плагины делают, пока окно живёт: клавиши, таймеры, настройки.

use std::time::{Duration, Instant};

use dimaster_config::{Chord, Effect, Snapshot};
use dimaster_i18n::t;

use crate::settings;
use crate::view::View;

// сколько строк истории видит обработчик
const HISTORY_LINES: u64 = 1000;
// столько разных таблиц `dimaster.set` помнится одновременно
const MAX_OVERRIDES: usize = 64;

#[derive(Debug)]
struct Timer {
    due: Instant,
    period: Duration,
    index: usize,
}

/// Привязки, расписание и настройки, установленные плагинами.
#[derive(Debug, Default)]
pub(crate) struct Hooks {
    keys: Vec<Chord>,
    timers: Vec<Timer>,
    // литералы Lua в порядке установки
    pub(crate) overrides: Vec<String>,
}

// повторная установка той же таблицы переносит её в конец, а не копит
fn remember(overrides: &mut Vec<String>, literal: String) {
    overrides.retain(|old| *old != literal);
    overrides.push(literal);
    if overrides.len() > MAX_OVERRIDES {
        overrides.remove(0);
    }
}

impl View {
    /// После загрузки плагинов: что они привязали и запланировали.
    pub(crate) fn collect_hooks(&mut self) {
        let Some(plugins) = &self.plugins else { return };
        let now = Instant::now();
        self.hooks.keys = plugins.keys();
        let periods = plugins.timers().into_iter().enumerate();
        self.hooks.timers = periods
            .map(|(index, seconds)| {
                let period = Duration::from_secs_f64(seconds.clamp(1.0, 86_400.0));
                Timer {
                    due: now + period,
                    period,
                    index,
                }
            })
            .collect();
    }

    // история собирается только там, где обработчик может её спросить
    pub(crate) fn full_snapshot(&self) -> Snapshot {
        let range = self.term.line_range();
        let from = range.end.saturating_sub(HISTORY_LINES).max(range.start);
        let lines: Vec<String> = (from..range.end)
            .filter_map(|line| self.term.line(line).map(|row| row.text()))
            .collect();
        Snapshot {
            history: lines.join("\n").trim_end().to_owned(),
            ..self.plugin_snapshot()
        }
    }

    /// `true` — нажатие забрал плагин.
    pub(crate) fn plugin_key(&mut self, pressed: &[Chord]) -> bool {
        let found = pressed
            .iter()
            .find_map(|chord| self.hooks.keys.iter().position(|k| k == chord));
        let (Some(index), Some(plugins)) = (found, &self.plugins) else {
            return false;
        };
        let (consumed, effects) = plugins.key(index, self.full_snapshot());
        self.apply_effects(effects);
        consumed
    }

    /// Выполняет созревшие таймеры и возвращает время следующего.
    pub(crate) fn run_timers(&mut self) -> Option<Instant> {
        let now = Instant::now();
        let mut due = Vec::new();
        for timer in self.hooks.timers.iter_mut().filter(|t| t.due <= now) {
            // пропущенные во сне срабатывания не догоняются
            timer.due = now + timer.period;
            due.push(timer.index);
        }
        for index in due {
            let Some(plugins) = &self.plugins else { break };
            let effects = plugins.tick(index, self.full_snapshot());
            self.apply_effects(effects);
        }
        self.hooks.timers.iter().map(|t| t.due).min()
    }

    pub(crate) fn plugin_command(&mut self, command: &str, exit: i32, seconds: f64) {
        let Some(plugins) = &self.plugins else { return };
        let effects = plugins.command_finished(command, exit, seconds, self.full_snapshot());
        self.apply_effects(effects);
    }

    /// `dimaster.set` и `dimaster.reset`; ошибочная таблица не принимается.
    pub(crate) fn plugin_settings(&mut self, effect: Effect) {
        let mut overrides = self.hooks.overrides.clone();
        match effect {
            Effect::Set(literal) => remember(&mut overrides, literal),
            Effect::Reset => overrides.clear(),
            _ => return,
        }
        match settings::read_with(&overrides) {
            Ok(config) => {
                self.hooks.overrides = overrides;
                if config != self.config {
                    self.apply(config);
                }
            }
            Err(e) => log::warn!("{}", t!("Ошибка конфига: {error}", error = e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_override_is_not_stored_twice() {
        let mut list = Vec::new();
        for item in ["a", "b", "a"] {
            remember(&mut list, item.into());
        }
        assert_eq!(list, ["b", "a"]);
        (0..100).for_each(|i| remember(&mut list, i.to_string()));
        assert_eq!(list.len(), MAX_OVERRIDES);
    }
}
