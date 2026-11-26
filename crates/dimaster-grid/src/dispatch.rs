//! Связывает команды парсера с операциями над экраном.

use dimaster_vt::{Perform, Sequence};

use crate::terminal::{Size, State};
use crate::{Cell, sgr};

impl Perform for State {
    fn print(&mut self, ch: char) {
        self.put(ch);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x08 => self.set_col(self.cursor.col.saturating_sub(1)),
            b'\t' => self.tab(1, true),
            b'\n' | 0x0B | 0x0C => self.line_feed(),
            b'\r' => self.set_col(0),
            // SO и SI переключают наборы G1/G0
            0x0E => self.cursor.charsets.active = 1,
            0x0F => self.cursor.charsets.active = 0,
            _ => {}
        }
    }

    fn esc(&mut self, seq: Sequence<'_>) {
        match (seq.intermediates, seq.action) {
            ([slot @ (b'(' | b')')], set) => {
                self.cursor.charsets.graphics[usize::from(*slot == b')')] = set == b'0';
            }
            ([b'#'], b'8') => self.alignment_test(),
            ([], action) => self.plain_esc(action),
            _ => log::debug!("неизвестный ESC: {seq:?}"),
        }
    }

    fn csi(&mut self, seq: Sequence<'_>) {
        match seq.intermediates {
            [] => self.standard_csi(&seq),
            [b'?'] => self.private_csi(&seq),
            // вторичный DA: объявляем себя VT220-совместимым
            [b'>'] if seq.action == b'c' => self.reply.extend_from_slice(b"\x1b[>1;10;0c"),
            _ => log::debug!("неизвестный CSI: {seq:?}"),
        }
    }

    fn osc(&mut self, data: &[u8]) {
        self.handle_osc(data);
    }
}

impl State {
    fn plain_esc(&mut self, action: u8) {
        match action {
            b'7' => self.save_cursor(),
            b'8' => self.restore_cursor(),
            b'D' => self.line_feed(),
            b'E' => {
                self.set_col(0);
                self.line_feed();
            }
            b'H' => self.tabs[self.cursor.col] = true,
            b'M' => self.reverse_line_feed(),
            b'c' => self.reset(),
            b'=' => self.modes.app_keypad = true,
            b'>' => self.modes.app_keypad = false,
            _ => log::debug!("неизвестный ESC {action:#x}"),
        }
    }

    // RIS сбрасывает всё, кроме накопленной истории
    fn reset(&mut self) {
        let size = Size {
            rows: self.screen.rows.len(),
            cols: self.screen.cols(),
        };
        self.set_alternate(false);
        let history = std::mem::take(&mut self.screen.history);
        let reply = std::mem::take(&mut self.reply);
        *self = Self::new(size);
        self.screen.history = history;
        self.reply = reply;
    }

    // DECALN: vttest проверяет им геометрию экрана
    fn alignment_test(&mut self) {
        let cell = Cell::new('E', Default::default());
        self.screen
            .rows
            .iter_mut()
            .for_each(|r| r.cells.fill(cell.clone()));
    }

    fn standard_csi(&mut self, seq: &Sequence<'_>) {
        let n = usize::from(seq.params.get_or(0, 1));
        match seq.action {
            b'A'..=b'H' | b'a' | b'd' | b'e' | b'f' | b'`' => self.cursor_csi(seq, n),
            b'J' | b'K' | b'L' | b'M' | b'P' | b'S' | b'T' | b'X' | b'@' | b'b' => {
                self.edit_csi(seq, n);
            }
            b'm' => sgr::apply(&mut self.cursor.style, seq.params),
            _ => self.control_csi(seq, n),
        }
    }

    fn cursor_csi(&mut self, seq: &Sequence<'_>, n: usize) {
        let delta = n as isize;
        match seq.action {
            b'A' => self.move_vertical(-delta),
            b'B' | b'e' => self.move_vertical(delta),
            b'C' | b'a' => self.set_col(self.cursor.col + n),
            b'D' => self.set_col(self.cursor.col.saturating_sub(n)),
            b'E' | b'F' => {
                self.move_vertical(if seq.action == b'E' { delta } else { -delta });
                self.set_col(0);
            }
            b'G' | b'`' => self.set_col(n - 1),
            b'H' | b'f' => self.goto(n - 1, usize::from(seq.params.get_or(1, 1)) - 1),
            b'd' => self.goto(n - 1, self.cursor.col),
            _ => {}
        }
    }

    fn edit_csi(&mut self, seq: &Sequence<'_>, n: usize) {
        let mode = seq.params.get_or(0, 0);
        let fill = self.blank();
        match seq.action {
            b'J' => self.erase_display(mode),
            b'K' => self.erase_line(mode),
            b'L' => self.shift_lines(n, true),
            b'M' => self.shift_lines(n, false),
            b'P' => self.delete_chars(n),
            b'S' => self.screen.scroll_up(n, &fill),
            b'T' => self.screen.scroll_down(n, &fill),
            b'X' => self.erase_cells(self.cursor.col, self.cursor.col + n),
            b'@' => self.insert_blank(n),
            b'b' => self.repeat(n),
            _ => {}
        }
    }

    fn repeat(&mut self, count: usize) {
        let Some(ch) = self.last_char else { return };
        // больше экрана повторять незачем — защита от CSI 65535 b в цикле
        let limit = self.screen.rows.len() * self.screen.cols();
        (0..count.min(limit)).for_each(|_| self.put(ch));
    }

    fn control_csi(&mut self, seq: &Sequence<'_>, n: usize) {
        match seq.action {
            b'I' => self.tab(n, true),
            b'Z' => self.tab(n, false),
            b'g' => self.clear_tabs(seq.params.get_or(0, 0)),
            b'h' | b'l' if seq.params.get_or(0, 0) == 4 => self.modes.insert = seq.action == b'h',
            b'r' => {
                let rows = self.screen.rows.len() as u16;
                let bottom = usize::from(seq.params.get_or(1, rows));
                self.set_scroll_region(n - 1, bottom - 1);
            }
            b's' => self.save_cursor(),
            b'u' => self.restore_cursor(),
            b'c' => self.reply.extend_from_slice(b"\x1b[?62;22c"),
            b'n' => self.status_report(seq.params.get_or(0, 0)),
            _ => log::debug!("неизвестный CSI: {seq:?}"),
        }
    }

    fn clear_tabs(&mut self, mode: u16) {
        match mode {
            0 => self.tabs[self.cursor.col] = false,
            3 => self.tabs.fill(false),
            _ => {}
        }
    }

    fn status_report(&mut self, kind: u16) {
        match kind {
            5 => self.reply.extend_from_slice(b"\x1b[0n"),
            6 => {
                let base = if self.modes.origin {
                    self.screen.top
                } else {
                    0
                };
                let (row, col) = (self.cursor.row - base.min(self.cursor.row), self.cursor.col);
                let report = format!("\x1b[{};{}R", row + 1, col + 1);
                self.reply.extend_from_slice(report.as_bytes());
            }
            _ => {}
        }
    }

    fn private_csi(&mut self, seq: &Sequence<'_>) {
        if !matches!(seq.action, b'h' | b'l') {
            return log::debug!("неизвестный приватный CSI: {seq:?}");
        }
        let on = seq.action == b'h';
        for mode in seq.params.groups().filter_map(|g| g.first().copied()) {
            self.set_private_mode(mode, on);
        }
    }

    fn set_private_mode(&mut self, mode: u16, on: bool) {
        match mode {
            6 => {
                self.modes.origin = on;
                self.goto(0, 0);
            }
            47 | 1047 => self.set_alternate(on),
            1048 if on => self.save_cursor(),
            1048 => self.restore_cursor(),
            1049 if on => {
                self.save_cursor();
                self.set_alternate(true);
            }
            1049 => {
                self.set_alternate(false);
                self.restore_cursor();
            }
            _ => self.modes.set_private(mode, on),
        }
    }
}
