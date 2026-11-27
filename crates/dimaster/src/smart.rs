//! Реакция окна на команды: свечение рамки, уведомление, Sudo Fix.

use dimaster_i18n::tr;
use std::time::{Duration, Instant};

use dimaster_render::Rect;

use crate::desktop;
use crate::view::View;

/// Что окно помнит между командами.
#[derive(Debug, Default)]
pub(crate) struct Assist {
    seen: u64,
    /// Листание истории стрелками.
    pub(crate) walk: crate::native::Walk,
    pub(crate) glow_since: Option<Instant>,
    /// Пользователь что-то набрал после последней команды.
    pub(crate) typed: bool,
    last_alt: Option<Instant>,
}

impl Assist {
    /// `true` на втором чистом нажатии Alt.
    pub(crate) fn alt_tap(&mut self, now: Instant, window: Duration) -> bool {
        let double = self
            .last_alt
            .is_some_and(|t| now.duration_since(t) < window);
        self.last_alt = (!double).then_some(now);
        double
    }

    pub(crate) fn other_key(&mut self) {
        self.last_alt = None;
    }
}

// ширина рамки плавно растёт и убывает
fn pulse_width(elapsed: f32, period: f32) -> u32 {
    let wave = (elapsed / period * std::f32::consts::TAU).sin();
    let (middle, swing) = (
        dimaster_limits::GLOW_PX.u32() as f32,
        dimaster_limits::GLOW_WAVE_PX.u32() as f32,
    );
    (middle + wave * swing).round().max(0.0) as u32
}

fn frame(width: u32, height: u32, thickness: u32, color: u32) -> [Rect; 4] {
    let rect = |x, y, w, h| Rect {
        x,
        y,
        width: w,
        height: h,
        color,
        translucent: true,
        radius: 0,
        opacity: 1.0,
        stroke: None,
    };
    [
        rect(0, 0, width, thickness),
        rect(0, height.saturating_sub(thickness), width, thickness),
        rect(0, 0, thickness, height),
        rect(width.saturating_sub(thickness), 0, thickness, height),
    ]
}

impl View {
    /// Вызывается после разбора вывода: замечает новые завершённые команды.
    pub(crate) fn check_commands(&mut self) {
        let completed = self.term.completed_commands();
        if completed == self.assist.seen {
            return;
        }
        self.assist.seen = completed;
        self.assist.typed = false;
        // после команды стрелки листают историю заново
        self.assist.walk = crate::native::Walk::default();
        let Some(last) = self.term.commands().next_back() else {
            return;
        };
        let last = last.clone();
        self.help_with(&last);
        let (command, exit, seconds) = (last.command, last.exit, last.duration);
        if exit != 127 {
            self.history_store
                .record_command(&command, self.config.tuning.history_commands);
        }
        self.plugin_command(&command, exit, seconds.as_secs_f64());
        let limit = self.config.smart.long_command;
        if limit > 0.0 && !self.focused && seconds.as_secs_f32() >= limit {
            self.long_command_finished(&command, exit, seconds);
        }
    }

    fn long_command_finished(&mut self, command: &str, exit: i32, took: Duration) {
        if self.config.smart.glow {
            self.assist.glow_since = Some(Instant::now());
        }
        if self.config.smart.notify {
            let title = match exit {
                0 => "Команда завершена",
                _ => "Команда завершилась с ошибкой",
            };
            let body = format!("{command} — {} {}", took.as_secs(), tr("с"));
            desktop::notify(&tr(title), &body);
        }
    }

    pub(crate) fn glow_rects(&self) -> Vec<Rect> {
        let Some(since) = self.assist.glow_since else {
            return Vec::new();
        };
        let size = self.window.inner_size();
        let thickness = pulse_width(
            since.elapsed().as_secs_f32(),
            self.config.tuning.glow_period as f32 / 1000.0,
        );
        frame(
            size.width,
            size.height,
            thickness,
            self.config.smart.glow_color,
        )
        .to_vec()
    }

    /// Добавляет `sudo`: к набранной строке или к прошлой команде.
    pub(crate) fn sudo_fix(&mut self) {
        if self.term.is_alternate() {
            return;
        }
        if self.assist.typed {
            // Ctrl+A, «sudo », Ctrl+E — понимают readline, zle и fish
            return self.type_bytes(b"\x01sudo \x05".to_vec());
        }
        let last = self.term.commands().next_back().map(|c| c.command.clone());
        match last.filter(|c| !c.is_empty() && !c.starts_with("sudo ")) {
            Some(command) => self.type_bytes(format!("sudo {command}").into_bytes()),
            None => self.type_bytes(b"sudo ".to_vec()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_alt_needs_two_clean_taps() {
        let (mut assist, now) = (Assist::default(), Instant::now());
        let window = Duration::from_millis(350);
        assert!(!assist.alt_tap(now, window));
        assert!(assist.alt_tap(now + Duration::from_millis(100), window));
        assert!(!assist.alt_tap(now + Duration::from_millis(150), window));
        assist.other_key();
        assert!(!assist.alt_tap(now + Duration::from_millis(200), window));
        assert!((1..=7).contains(&pulse_width(0.4, 1.6)));
    }
}
