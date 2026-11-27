//! Витрина скинов в окне: показ, предпросмотр на лету и сохранение выбора.
//!
//! Выбор лежит в `~/.local/share/dimaster/skin` и накладывается поверх
//! настроек, поэтому файлы конфига не меняются.

use std::path::PathBuf;

use dimaster_config::{Theme, skin_literal, skins};
use dimaster_i18n::{t, tr};
use unicode_width::UnicodeWidthStr;

use crate::history_store::data_dir;
use crate::modes::skins::{Outcome, ROWS, Skins};
use crate::modes::{Mode, Press};
use crate::overlay::Drawn;
use crate::settings;
use crate::view::View;

const COLUMNS: usize = 46;
// цвета ANSI, которые видны в образце строки
const SWATCH: [usize; 6] = [1, 3, 2, 6, 4, 5];

fn file() -> Option<PathBuf> {
    Some(data_dir()?.join("skin"))
}

/// Имя выбранного скина.
pub(crate) fn chosen() -> Option<String> {
    let name = std::fs::read_to_string(file()?).ok()?;
    Some(name.trim().to_owned()).filter(|n| !n.is_empty())
}

/// Цвета выбранного скина как первый слой поверх настроек.
pub(crate) fn chosen_literal() -> Option<String> {
    skin_literal(&chosen()?)
}

fn store(name: Option<&str>) {
    let Some(file) = file() else { return };
    let result = match name {
        Some(name) => file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&file, name)),
        None => match std::fs::remove_file(&file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        },
    };
    if let Err(e) = result {
        log::warn!("{}", t!("Скин не сохранён: {error}", error = e));
    }
}

impl View {
    pub(crate) fn open_skins(&mut self) {
        let mut names = vec![tr("Как в настройках")];
        names.extend(skins().into_iter().map(|s| s.name.to_owned()));
        let current = chosen().and_then(|c| names.iter().position(|n| *n == c));
        let saved = current.unwrap_or(0);
        let mut state = Skins {
            names,
            selected: saved,
            saved,
            ..Skins::default()
        };
        state.show_selected();
        // тема без скина нужна для первой строки списка
        let plain = settings::read_plain(&self.hooks.overrides);
        self.plain_theme = plain.map_or(self.config.theme, |c| c.theme);
        self.mode = Mode::Skins(state);
        self.window.request_redraw();
    }

    fn skin_theme(&self, index: usize) -> Theme {
        let skin = index
            .checked_sub(1)
            .and_then(|i| skins().into_iter().nth(i));
        skin.map_or(self.plain_theme, |s| s.theme)
    }

    fn paint_skin(&mut self, index: usize) {
        let theme = self.skin_theme(index);
        if theme != self.config.theme {
            self.config.theme = theme;
            self.apply_to_terminal();
            self.renderer.reset_glyphs();
        }
        self.window.request_redraw();
    }

    pub(crate) fn skin_press(&mut self, mut state: Skins, press: Press) {
        let outcome = state.press(press);
        self.skin_outcome(state, outcome);
    }

    /// Выполняет решение витрины и оставляет её открытой, если нужно.
    pub(crate) fn skin_outcome(&mut self, mut state: Skins, outcome: Outcome) {
        match outcome {
            Outcome::Preview => self.paint_skin(state.selected),
            Outcome::Cancel => return self.paint_skin(state.saved),
            Outcome::Keep(close) => {
                self.paint_skin(state.selected);
                let name = state
                    .names
                    .get(state.selected)
                    .filter(|_| state.selected > 0);
                store(name.map(String::as_str));
                state.saved = state.selected;
                if close {
                    return;
                }
            }
        }
        self.mode = Mode::Skins(state);
    }

    fn skins_corner(&self, rows: usize) -> (u32, u32) {
        let size = self.window.inner_size();
        let (width, height) = self.panel_size(COLUMNS, rows);
        let top = self.title_height() + self.fonts.metrics().cell_height;
        let y = top.min(size.height.saturating_sub(height));
        (size.width.saturating_sub(width) / 2, y)
    }

    fn skin_lines(&self, state: &Skins) -> Vec<(String, bool)> {
        let header = t!(" Скины: {q}_", q = state.query);
        let mut lines = vec![(header, false)];
        let found = state.matches().into_iter().skip(state.top).take(ROWS);
        lines.extend(found.map(|index| {
            let mark = if index == state.saved { '●' } else { ' ' };
            (
                format!(" {mark} {}", state.names[index]),
                index == state.selected,
            )
        }));
        lines.push((tr(" клик или Enter — выбрать · Esc — закрыть"), false));
        lines
    }

    // образец цветов справа в каждой строке со скином
    fn swatches(&self, state: &Skins, corner: (u32, u32), out: &mut Drawn) {
        let metrics = self.fonts.metrics();
        let (cell, line) = (metrics.cell_width, metrics.cell_height);
        let pad = self.config.ui.panel.padding;
        let used = state.names.iter().map(|n| n.width()).max().unwrap_or(0);
        let column = (used + 5).min(COLUMNS - SWATCH.len() - 2) as u32;
        let found = state.matches().into_iter().skip(state.top).take(ROWS);
        for (row, index) in found.enumerate() {
            let theme = self.skin_theme(index);
            let y = corner.1 + pad + (row as u32 + 1) * line + line / 6;
            let x = corner.0 + pad + column * cell;
            let size = (cell, line - line / 3);
            out.rects
                .push(self.plain_rect(x, y, (cell * 7, size.1), theme.background));
            for (i, &ansi) in SWATCH.iter().enumerate() {
                let at = x + i as u32 * cell + cell / 2;
                out.rects
                    .push(self.plain_rect(at, y, size, theme.ansi[ansi]));
            }
        }
    }

    pub(crate) fn skins_panel(&self) -> Drawn {
        let Mode::Skins(state) = &self.mode else {
            return Drawn::default();
        };
        let lines = self.skin_lines(state);
        let corner = self.skins_corner(lines.len());
        let mut out = self.panel(corner, COLUMNS, &lines);
        self.swatches(state, corner, &mut out);
        out
    }

    // какая строка списка под указателем
    fn skin_row(&self, state: &Skins) -> Option<usize> {
        let rows = state.matches().len().saturating_sub(state.top).min(ROWS) + 2;
        let corner = self.skins_corner(rows);
        let (width, height) = self.panel_size(COLUMNS, rows);
        let (x, y) = self.pointer.position;
        let inside = x >= f64::from(corner.0)
            && x < f64::from(corner.0 + width)
            && y < f64::from(corner.1 + height);
        let top = corner.1 + self.config.ui.panel.padding + self.fonts.metrics().cell_height;
        let offset = y - f64::from(top);
        (inside && offset >= 0.0)
            .then(|| (offset / f64::from(self.fonts.metrics().cell_height)) as usize)
    }

    /// Щелчок при открытой витрине: по строке — выбрать, мимо — закрыть.
    pub(crate) fn skins_click(&mut self) -> bool {
        if !matches!(self.mode, Mode::Skins(_)) {
            return false;
        }
        let Mode::Skins(mut state) = std::mem::take(&mut self.mode) else {
            return false;
        };
        let outcome = self.skin_row(&state).and_then(|row| state.click(row));
        self.skin_outcome(state, outcome.unwrap_or(Outcome::Cancel));
        self.render_status();
        true
    }
}
