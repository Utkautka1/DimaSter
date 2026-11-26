//! Расширения OSC: форма указателя, индикатор выполнения, команды ConEmu.

use crate::terminal::State;
use crate::untrusted::clean;

/// Ход долгой операции (OSC 9;4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    // проценты, 0–100
    pub value: u8,
    pub failed: bool,
    pub paused: bool,
}

/// Что программам разрешено менять; задаёт пользователь.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub title: bool,
    pub colors: bool,
    pub cursor_shape: bool,
    pub pointer_shape: bool,
    pub progress: bool,
    pub notifications: bool,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            title: true,
            colors: true,
            cursor_shape: true,
            pointer_shape: true,
            progress: true,
            notifications: true,
        }
    }
}

// `state;value`: 0 убрать, 1 идёт, 2 ошибка, 3 без процентов, 4 пауза
fn progress(value: &str) -> Option<Progress> {
    let mut parts = value.split(';');
    let state: u8 = parts.next()?.parse().ok()?;
    let percent = parts.next().and_then(|v| v.parse::<u8>().ok()).unwrap_or(0);
    (1..=4).contains(&state).then_some(Progress {
        value: if state == 3 { 100 } else { percent.min(100) },
        failed: state == 2,
        paused: state == 4,
    })
}

impl State {
    /// OSC 22: имя формы указателя из CSS (`text`, `pointer`, `crosshair`…).
    pub(crate) fn pointer_shape(&mut self, name: &str) {
        let allowed = self.policy.pointer_shape && !name.is_empty() && name.len() <= 32;
        self.pointer = allowed.then(|| clean(name));
    }

    /// OSC 9: уведомление или команда ConEmu `9;<номер>;…`.
    pub(crate) fn conemu(&mut self, body: &str) {
        let (code, rest) = body.split_once(';').unwrap_or((body, ""));
        match code.parse::<u8>() {
            Ok(4) if self.policy.progress => self.progress = progress(rest),
            Ok(9) => self.cwd = Some(clean(rest.trim_matches('"'))),
            Ok(2) => self.notify("DimaSter", rest),
            // паузы, заголовки вкладок и прочее из Windows не исполняются
            Ok(1..=12) if !rest.is_empty() => {}
            _ => self.notify("DimaSter", body),
        }
    }
}
