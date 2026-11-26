//! Входы в режимы и обработка клавиш внутри них.

use dimaster_grid::Point;
use dimaster_i18n::t;
use winit::keyboard::Key;
use winit::window::Fullscreen;

use crate::clipboard::Target;
use crate::desktop;
use crate::keys;
use crate::modes::{Mode, Press, hint, history, minimap, palette, paste, press, search, vi};
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
            custom: self.config.ui.palette.items.clone(),
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
            self.render_status();
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
            Mode::Hint(state) => self.hint_press(state, press),
            Mode::History(state) => self.history_press(state, press),
            Mode::Minimap(state) => self.minimap_press(state, press),
            Mode::Paste(state) => self.paste_press(state, press),
            Mode::CloseConfirm => self.close_press(press),
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

    pub(crate) fn open_hints(&mut self) {
        let hints = hint::Hints::collect(&self.term, self.scroll);
        if !hints.targets.is_empty() {
            self.mode = Mode::Hint(hints);
            self.window.request_redraw();
        }
    }

    pub(crate) fn open_history(&mut self) {
        let entries = self.term.commands().rev().cloned().collect();
        self.mode = Mode::History(history::History {
            entries,
            selected: 0,
        });
        self.window.request_redraw();
    }

    fn hint_press(&mut self, mut state: hint::Hints, press: Press) {
        match state.press(press) {
            hint::Outcome::Stay => self.mode = Mode::Hint(state),
            hint::Outcome::Open(link) => desktop::open(&link, self.term.cwd()),
            hint::Outcome::Copy(text) => self.clipboard.set(Target::Clipboard, text),
            hint::Outcome::Close => {}
        }
    }

    fn history_press(&mut self, mut state: history::History, press: Press) {
        match state.press(press) {
            history::Outcome::Stay => self.mode = Mode::History(state),
            history::Outcome::Insert(command) => self.type_bytes(command.into_bytes()),
            history::Outcome::CopyOutput(text) => self.clipboard.set(Target::Clipboard, text),
            history::Outcome::Close => {}
        }
    }

    pub(crate) fn open_minimap(&mut self) {
        self.mode = Mode::Minimap(minimap::Minimap {
            origin: self.scroll,
        });
        self.window.request_redraw();
    }

    fn minimap_press(&mut self, state: minimap::Minimap, press: Press) {
        let max = self.term.screen().history_len();
        match state.press(press, self.scroll, max, self.term.screen().rows().len()) {
            minimap::Outcome::Scroll(scroll) => {
                self.scroll = scroll;
                self.mode = Mode::Minimap(state);
            }
            minimap::Outcome::Accept => {}
            minimap::Outcome::Cancel => self.scroll = state.origin,
        }
    }

    fn paste_press(&mut self, state: paste::PasteConfirm, press: Press) {
        let bracketed = self.term.modes().bracketed_paste;
        match state.press(press) {
            paste::Outcome::Stay => self.mode = Mode::Paste(state),
            paste::Outcome::PasteRaw => self.type_bytes(keys::paste(&state.text, bracketed)),
            paste::Outcome::PasteSingleLine => {
                self.type_bytes(keys::paste_single_line(&state.text, bracketed))
            }
            paste::Outcome::Cancel => {}
        }
    }

    fn close_press(&mut self, press: Press) {
        match press {
            Press::Enter { .. } | Press::Char('y' | 'Y' | 'д' | 'Д') => self.closing = true,
            Press::Escape | Press::Ctrl('c') | Press::Char('n' | 'N' | 'н' | 'Н') => {}
            _ => self.mode = Mode::CloseConfirm,
        }
    }

    pub(crate) fn request_close(&mut self) -> bool {
        if self.config.confirm_close && !matches!(self.mode, Mode::CloseConfirm) {
            self.mode = Mode::CloseConfirm;
            self.window.request_redraw();
            return false;
        }
        true
    }

    // виджет включается до перечитывания конфига
    pub(crate) fn toggle_widget(&mut self, statusbar: bool) {
        let mut config = self.config.clone();
        match statusbar {
            true => config.widgets.statusbar = !config.widgets.statusbar,
            false => config.widgets.monitor = !config.widgets.monitor,
        }
        self.apply(config);
        self.render_status();
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
            None => log::debug!("{}", t!("выше нет меток приглашения (OSC 133)")),
        }
    }
}
