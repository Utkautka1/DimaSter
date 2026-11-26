//! Переключаемые режимы терминала (SM/RM и DECSET/DECRST).

/// Форма курсора (DECSCUSR).
use dimaster_i18n::t;
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    #[default]
    Block,
    Underline,
    Bar,
}

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
    /// Программа просит не показывать кадр, пока не дорисует (режим 2026).
    pub synchronized: bool,
    pub cursor_shape: CursorShape,
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
            synchronized: false,
            cursor_shape: CursorShape::Block,
        }
    }
}

impl Modes {
    /// Текущее значение режима; `None` — режим не поддерживается.
    pub(crate) fn private(&self, mode: u16) -> Option<bool> {
        Some(match mode {
            1 => self.app_cursor,
            7 => self.autowrap,
            25 => self.cursor_visible,
            1000 | 1002 | 1003 => self.mouse == mode,
            1004 => self.focus_events,
            1006 => self.sgr_mouse,
            2004 => self.bracketed_paste,
            2026 => self.synchronized,
            _ => return None,
        })
    }

    /// DECSCUSR: нечётные и чётные значения различаются только миганием.
    pub(crate) fn set_cursor_shape(&mut self, style: u16) {
        self.cursor_shape = match style {
            3 | 4 => CursorShape::Underline,
            5 | 6 => CursorShape::Bar,
            _ => CursorShape::Block,
        };
    }

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
            2026 => self.synchronized = on,
            _ => log::debug!(
                "{}",
                t!(
                    "неподдерживаемый DEC-режим {mode}={on}",
                    mode = mode,
                    on = on
                )
            ),
        }
    }
}
