//! Стек флагов клавиатурного протокола Kitty.

use dimaster_vt::Sequence;

use crate::terminal::State;

// поддерживаются все 5 флагов протокола Kitty (1|2|4|8|16)
const SUPPORTED: u16 = 0b1_1111;
/// Флаги основного и альтернативного экранов независимы.
#[derive(Debug, Default)]
pub(crate) struct KeyboardStack {
    stacks: [Vec<u16>; 2],
}

impl KeyboardStack {
    pub(crate) fn current(&self, alternate: bool) -> u16 {
        self.stacks[usize::from(alternate)]
            .last()
            .copied()
            .unwrap_or(0)
    }

    fn push(&mut self, alternate: bool, flags: u16) {
        let stack = &mut self.stacks[usize::from(alternate)];
        if stack.len() == dimaster_limits::KEYBOARD_STACK.size() {
            stack.remove(0);
        }
        stack.push(flags & SUPPORTED);
    }

    fn pop(&mut self, alternate: bool, count: u16) {
        let stack = &mut self.stacks[usize::from(alternate)];
        stack.truncate(stack.len().saturating_sub(usize::from(count)));
    }

    // режимы: 1 — заменить, 2 — добавить биты, 3 — убрать биты
    fn set(&mut self, alternate: bool, flags: u16, mode: u16) {
        let old = self.current(alternate);
        let new = match mode {
            2 => old | flags,
            3 => old & !flags,
            _ => flags,
        };
        self.pop(alternate, 1);
        self.push(alternate, new);
    }
}

impl State {
    /// Последовательности `CSI > u`, `CSI < u`, `CSI = u` и `CSI ? u`.
    pub(crate) fn keyboard_csi(&mut self, prefix: u8, seq: &Sequence<'_>) {
        let first = seq.params.get_or(0, 0);
        match prefix {
            b'>' => self.keyboard.push(self.alternate, first),
            b'<' => self.keyboard.pop(self.alternate, first.max(1)),
            b'=' => self
                .keyboard
                .set(self.alternate, first, seq.params.get_or(1, 1)),
            _ => {
                let flags = self.keyboard.current(self.alternate);
                self.answer(format!("\x1b[?{flags}u"));
            }
        }
    }
}
