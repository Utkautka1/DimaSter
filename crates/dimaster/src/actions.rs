//! Входы в режимы и обработка клавиш внутри них.

use dimaster_grid::Point;
use winit::keyboard::Key;
use winit::window::Fullscreen;

use crate::clipboard::Target;
use crate::modes::{Mode, Press, palette, press, search, vi};
use crate::view::View;

impl View {
    pub(crate) fn open_search(&mut self) {
        self.mode = Mode::Search(Default::default());
        self.window.request_redraw();
    }

    pub(crate) fn open_vi(&mut self) {
        let cursor = self.term.cursor();
        let line = self.term.visible_line(0) + cursor.row as u64;
        self.mode = Mode::Vi(vi::Vi::new(Point {
            line,
            col: cursor.col,
        }));
        self.window.request_redraw();
    }

    pub(crate) fn open_palette(&mut self) {
        let plugins = self
            .plugins
            .as_ref()
            .map(|p| p.commands())
            .unwrap_or_default();
        let state = palette::Palette {
            plugins,
            ..Default::default()
        };
        self.mode = Mode::Palette(state);
        self.window.request_redraw();
    }

    /// `true` — клавиша принадлежит активному режиму.
    pub(crate) fn mode_key(&mut self, key: &Key, text: Option<&str>) -> bool {
        if matches!(self.mode, Mode::Normal) {
            return false;
        }
        if let Some(press) = press(key, text, self.modifiers) {
            self.mode_press(press);
            self.window.request_redraw();
        }
        true
    }

    // режим забирается из self на время обработки и возвращается, если не закрыт
    fn mode_press(&mut self, press: Press) {
        match std::mem::take(&mut self.mode) {
            Mode::Normal => {}
            Mode::Search(mut state) => {
                if state.press(press, &self.term) == search::Outcome::Show {
                    if let Some(found) = state.matches.get(state.current) {
                        self.reveal(found.start.line);
                    }
                    self.mode = Mode::Search(state);
                }
            }
            Mode::Vi(state) => self.vi_press(state, press),
            Mode::Palette(mut state) => match state.press(press) {
                palette::Outcome::Stay => self.mode = Mode::Palette(state),
                palette::Outcome::Run(action) => self.perform(action),
                palette::Outcome::Close => {}
            },
        }
    }

    fn vi_press(&mut self, mut state: vi::Vi, press: Press) {
        match state.press(press, &self.term) {
            vi::Outcome::Stay => {
                self.reveal(state.cursor.line);
                self.mode = Mode::Vi(state);
            }
            vi::Outcome::Yank => {
                self.selecting.selection = state.selection();
                self.copy(Target::Clipboard);
                self.selecting.clear();
                self.scroll = 0;
            }
            vi::Outcome::Search => self.open_search(),
            vi::Outcome::Exit => self.scroll = 0,
        }
    }

    /// Прокручивает так, чтобы строка стала видимой.
    pub(crate) fn reveal(&mut self, line: u64) {
        let rows = self.term.screen().rows().len() as u64;
        let first = self.term.visible_line(self.scroll);
        // нижнюю строку может закрывать панель поиска
        if line >= first && line + 1 < first + rows {
            return;
        }
        let bottom = self.term.visible_line(0);
        let wanted = line.saturating_sub(rows / 2);
        self.scroll = bottom.saturating_sub(wanted) as usize;
    }

    pub(crate) fn toggle_fullscreen(&self) {
        let full = self.window.fullscreen().is_none();
        self.window
            .set_fullscreen(full.then_some(Fullscreen::Borderless(None)));
    }

    /// Ставит предыдущее или следующее приглашение в верх экрана.
    pub(crate) fn jump_prompt(&mut self, forward: bool) {
        let top = self.term.visible_line(self.scroll);
        match self.term.prompt_line(top, forward) {
            Some(line) => {
                self.scroll = self.term.visible_line(0).saturating_sub(line) as usize;
                self.window.request_redraw();
            }
            None if forward => self.scroll_by(isize::MIN),
            None => log::debug!("выше нет меток приглашения (OSC 133)"),
        }
    }
}
