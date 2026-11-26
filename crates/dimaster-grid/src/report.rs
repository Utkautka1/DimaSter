//! Ответы на запросы программ о возможностях и состоянии терминала.
//!
//! nvim, yazi и tmux спрашивают об этом при старте; без ответа они ждут
//! таймаут или отключают часть функций.

use dimaster_i18n::t;
use dimaster_vt::Sequence;

use crate::terminal::State;

impl State {
    pub(crate) fn answer(&mut self, text: String) {
        self.reply.extend_from_slice(text.as_bytes());
    }

    /// DECRQM: 1 — режим включён, 2 — выключен, 0 — неизвестен.
    pub(crate) fn report_mode(&mut self, seq: &Sequence<'_>) {
        let mode = seq.params.get_or(0, 0);
        let state = match self.mode_state(mode) {
            Some(true) => 1,
            Some(false) => 2,
            None => 0,
        };
        self.answer(format!("\x1b[?{mode};{state}$y"));
    }

    fn mode_state(&self, mode: u16) -> Option<bool> {
        match mode {
            6 => Some(self.modes.origin),
            47 | 1047 | 1049 => Some(self.alternate),
            _ => self.modes.private(mode),
        }
    }

    pub(crate) fn report_version(&mut self) {
        self.answer(format!(
            "\x1bP>|DimaSter {}\x1b\\",
            env!("CARGO_PKG_VERSION")
        ));
    }

    /// XTWINOPS: размеры в пикселях и ячейках (нужны для показа картинок).
    pub(crate) fn report_size(&mut self, kind: u16) {
        let (rows, cols) = (self.screen.rows.len(), self.screen.cols());
        let (width, height) = self.cell_pixels;
        match kind {
            14 => self.answer(format!("\x1b[4;{};{}t", rows * height, cols * width)),
            16 => self.answer(format!("\x1b[6;{height};{width}t")),
            18 => self.answer(format!("\x1b[8;{rows};{cols}t")),
            _ => log::debug!("{}", t!("неподдерживаемый XTWINOPS {kind}", kind = kind)),
        }
    }
}
