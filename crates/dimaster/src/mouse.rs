//! Кодирование событий мыши для программ (режимы 1000, 1002, 1003, SGR).

use dimaster_grid::Modes;
use winit::keyboard::ModifiersState;

/// Кнопка в нумерации xterm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Button {
    Left = 0,
    Middle = 1,
    Right = 2,
    /// Движение без нажатых кнопок.
    None = 3,
    WheelUp = 64,
    WheelDown = 65,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Press,
    Release,
    Motion,
}

/// Событие в координатах ячеек (с нуля).
#[derive(Debug, Clone, Copy)]
pub(crate) struct MouseEvent {
    pub(crate) button: Button,
    pub(crate) action: Action,
    pub(crate) col: usize,
    pub(crate) row: usize,
}

/// `None`, если в текущем режиме программа такое событие не ждёт.
pub(crate) fn encode(event: MouseEvent, mods: ModifiersState, modes: Modes) -> Option<Vec<u8>> {
    let wanted = match event.action {
        Action::Motion => {
            modes.mouse == 1003 || modes.mouse == 1002 && event.button != Button::None
        }
        _ => modes.mouse != 0,
    };
    if !wanted {
        return None;
    }
    let motion = if event.action == Action::Motion {
        32
    } else {
        0
    };
    let code = event.button as u8
        + motion
        + 8 * u8::from(mods.alt_key())
        + 16 * u8::from(mods.control_key());
    Some(if modes.sgr_mouse {
        sgr(code, event)
    } else {
        legacy(code, event)
    })
}

fn sgr(code: u8, event: MouseEvent) -> Vec<u8> {
    let end = if event.action == Action::Release {
        'm'
    } else {
        'M'
    };
    format!("\x1b[<{code};{};{}{end}", event.col + 1, event.row + 1).into_bytes()
}

// старый формат: всё смещено на 32, отпускание не различает кнопки
fn legacy(code: u8, event: MouseEvent) -> Vec<u8> {
    let code = if event.action == Action::Release {
        (code & !3) | 3
    } else {
        code
    };
    let byte = |value: usize| u8::try_from(value + 33).unwrap_or(u8::MAX);
    vec![
        0x1b,
        b'[',
        b'M',
        code + 32,
        byte(event.col),
        byte(event.row),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(button: Button, action: Action) -> MouseEvent {
        MouseEvent {
            button,
            action,
            col: 4,
            row: 1,
        }
    }

    #[test]
    fn sgr_and_legacy_formats() {
        let none = ModifiersState::empty();
        let mut modes = Modes {
            mouse: 1000,
            sgr_mouse: true,
            ..Modes::default()
        };
        let release = event(Button::Left, Action::Release);
        assert_eq!(encode(release, none, modes).unwrap(), b"\x1b[<0;5;2m");
        modes.sgr_mouse = false;
        assert_eq!(encode(release, none, modes).unwrap(), b"\x1b[M#%\"");
    }

    #[test]
    fn motion_depends_on_mode() {
        let none = ModifiersState::empty();
        let drag = event(Button::Left, Action::Motion);
        let hover = event(Button::None, Action::Motion);
        let modes = |mouse| Modes {
            mouse,
            sgr_mouse: true,
            ..Modes::default()
        };
        assert!(encode(drag, none, modes(1000)).is_none());
        assert!(encode(drag, none, modes(1002)).is_some());
        assert!(encode(hover, none, modes(1002)).is_none());
        assert!(encode(hover, none, modes(1003)).is_some());
    }
}
