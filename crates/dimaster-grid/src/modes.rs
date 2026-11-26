//! Переключаемые режимы терминала (SM/RM и DECSET/DECRST).

/// Состояние режимов, влияющих на печать и на будущий ввод.
#[derive(Debug, Clone, Copy)]
pub struct Modes {
    pub autowrap: bool,
    pub origin: bool,
    pub insert: bool,
    pub cursor_visible: bool,
    pub app_cursor: bool,
    pub app_keypad: bool,
    pub bracketed_paste: bool,
    pub focus_events: bool,
    /// 0 — мышь выключена, иначе номер режима (1000, 1002, 1003).
    pub mouse: u16,
    pub sgr_mouse: bool,
}

impl Default for Modes {
    fn default() -> Self {
        Self {
            autowrap: true,
            origin: false,
            insert: false,
            cursor_visible: true,
            app_cursor: false,
            app_keypad: false,
            bracketed_paste: false,
            focus_events: false,
            mouse: 0,
            sgr_mouse: false,
        }
    }
}

impl Modes {
    /// Простые DEC-режимы; режимы с побочными эффектами обрабатывает `Terminal`.
    pub(crate) fn set_private(&mut self, mode: u16, on: bool) {
        match mode {
            1 => self.app_cursor = on,
            7 => self.autowrap = on,
            25 => self.cursor_visible = on,
            1000 | 1002 | 1003 => self.mouse = if on { mode } else { 0 },
            1004 => self.focus_events = on,
            1006 => self.sgr_mouse = on,
            2004 => self.bracketed_paste = on,
            _ => log::debug!("неподдерживаемый DEC-режим {mode}={on}"),
        }
    }
}
