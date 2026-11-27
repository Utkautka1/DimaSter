//! Витрина в окне: палитры и стили, предпросмотр на лету и сохранение.
//!
//! Выбор лежит в `~/.local/share/dimaster/` (файлы `skin` и `style`) и
//! накладывается поверх настроек, поэтому файлы конфига не меняются.

use std::path::PathBuf;
use std::sync::Arc;

use dimaster_config::{Scene, Style, Theme, skin_literal, skins, style, style_bar, styles};
use dimaster_grid::Picture;
use dimaster_i18n::{t, tr};
use unicode_width::UnicodeWidthStr;

use crate::history_store::data_dir;
use crate::modes::skins::{Outcome, ROWS, Skins};
use crate::modes::{Mode, Press};
use crate::overlay::Drawn;
use crate::view::View;
use crate::{scenery, settings};

const COLUMNS: usize = 46;
// строки над списком: поиск и стиль
const HEADER: usize = 2;
// цвета ANSI, которые видны в образце строки
const SWATCH: [usize; 6] = [1, 3, 2, 6, 4, 5];

/// Текущий стиль окна и нарисованный для него фон.
#[derive(Default)]
pub(crate) struct Look {
    pub(crate) style: Option<&'static Style>,
    // выбран скин или стиль: приглашение красится под него
    pub(crate) painted: bool,
    // тема без скина: нужна, пока открыта витрина
    plain: Theme,
    // фон рисуется заново только при смене сцены или цветов
    picture: Option<(Scene, Theme, Arc<Picture>)>,
}

impl Look {
    pub(crate) fn saved() -> Self {
        Self {
            style: chosen("style").and_then(|id| style(&id)),
            painted: chosen("skin").is_some() || chosen("style").is_some(),
            ..Self::default()
        }
    }
}

fn file(name: &str) -> Option<PathBuf> {
    Some(data_dir()?.join(name))
}

// сохранённый выбор: `skin` или `style`
fn chosen(name: &str) -> Option<String> {
    let text = std::fs::read_to_string(file(name)?).ok()?;
    Some(text.trim().to_owned()).filter(|n| !n.is_empty())
}

// цвета и стиль как слои поверх настроек
fn layers(skin: Option<&str>, look: Option<&Style>) -> Vec<String> {
    let colors = skin.and_then(skin_literal);
    let shape = look.map(|s| s.literal.to_owned());
    colors.into_iter().chain(shape).collect()
}

/// Слои сохранённого выбора.
pub(crate) fn chosen_layers() -> Vec<String> {
    let look = chosen("style").and_then(|id| style(&id));
    layers(chosen("skin").as_deref(), look)
}

fn store(name: &str, value: Option<&str>) {
    let Some(file) = file(name) else { return };
    let result = match value {
        Some(value) => file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&file, value)),
        None => match std::fs::remove_file(&file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        },
    };
    if let Err(e) = result {
        log::warn!("{}", t!("Скин не сохранён: {error}", error = e));
    }
}

// имя скина и стиль по номерам витрины; нулевые — «как в настройках»
fn picked(state: &Skins, skin: usize, look: usize) -> (Option<&str>, Option<&'static Style>) {
    let name = state.names.get(skin).filter(|_| skin > 0);
    let look = look.checked_sub(1).and_then(|i| styles().get(i));
    (name.map(String::as_str), look)
}

// список с отмеченным сохранённым выбором
fn gallery() -> Skins {
    let mut names = vec![tr("Как в настройках")];
    names.extend(skins().into_iter().map(|s| s.name.to_owned()));
    let mut titles = vec![tr("Как в настройках")];
    titles.extend(styles().iter().map(|s| tr(s.title)));
    let skin = chosen("skin").and_then(|c| names.iter().position(|n| *n == c));
    let look = chosen("style").and_then(|c| styles().iter().position(|s| s.id == c));
    let (saved, saved_style) = (skin.unwrap_or(0), look.map_or(0, |i| i + 1));
    let mut state = Skins {
        names,
        selected: saved,
        saved,
        styles: titles,
        style: saved_style,
        saved_style,
        ..Skins::default()
    };
    state.show_selected();
    state
}

impl View {
    pub(crate) fn open_skins(&mut self) {
        let plain = settings::read_plain(&self.hooks.overrides);
        self.look.plain = plain.map_or(self.config.theme, |c| c.theme);
        self.mode = Mode::Skins(gallery());
        self.window.request_redraw();
    }

    /// Фон: нарисованный стилем или картинка из настроек.
    pub(crate) fn wallpaper(&mut self) -> Option<(Arc<Picture>, f32)> {
        let Some(look) = self.look.style.filter(|s| s.scene != Scene::None) else {
            return match self.look.style {
                Some(_) => None,
                None => settings::wallpaper(&self.config),
            };
        };
        let (scene, theme) = (look.scene, self.config.theme);
        let cached = self
            .look
            .picture
            .as_ref()
            .filter(|p| p.0 == scene && p.1 == theme);
        let picture = match cached {
            Some(found) => found.2.clone(),
            None => {
                let picture = settings::picture(scenery::paint(scene, &theme)?);
                self.look.picture = Some((scene, theme, picture.clone()));
                picture
            }
        };
        Some((picture, f32::from(look.shade) / 100.0))
    }

    /// Строка статуса стиля вместо `statusbar.lua`.
    pub(crate) fn apply_style_bar(&mut self) {
        let Some(plugins) = &self.plugins else { return };
        let source = self.look.style.map(|s| style_bar(s, &self.config.theme));
        if let Err(e) = plugins.set_bar(source.as_deref()) {
            log::warn!("statusbar: {e}");
        }
    }

    // показывает сочетание палитры и стиля целиком
    fn show_look(&mut self, state: &Skins, skin: usize, look: usize) {
        let (name, look) = picked(state, skin, look);
        let mut stack = layers(name, look);
        stack.extend(self.hooks.overrides.iter().cloned());
        match settings::read_plain(&stack) {
            Ok(config) => {
                self.look.style = look;
                self.look.painted = name.is_some() || look.is_some();
                self.apply(config);
                self.apply_style_bar();
                self.renderer.reset_glyphs();
                self.render_status();
            }
            Err(e) => log::warn!("{}", t!("Ошибка конфига: {error}", error = e)),
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
            Outcome::Preview => self.show_look(&state, state.selected, state.style),
            Outcome::Cancel => return self.show_look(&state, state.saved, state.saved_style),
            Outcome::Keep(close) => {
                self.show_look(&state, state.selected, state.style);
                let (name, look) = picked(&state, state.selected, state.style);
                store("skin", name);
                store("style", look.map(|s| s.id));
                (state.saved, state.saved_style) = (state.selected, state.style);
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
        let name = state.styles.get(state.style).cloned().unwrap_or_default();
        let mut lines = vec![
            (t!(" Скины: {q}_", q = state.query), false),
            (t!(" Стиль:  ◀  {name}  ▶", name = name), false),
        ];
        let found = state.matches().into_iter().skip(state.top).take(ROWS);
        lines.extend(found.map(|index| {
            let mark = if index == state.saved { '●' } else { ' ' };
            (
                format!(" {mark} {}", state.names[index]),
                index == state.selected,
            )
        }));
        lines.push((tr(" ←→ стиль · клик/Enter — выбрать · Esc"), false));
        lines
    }

    fn skin_theme(&self, index: usize) -> Theme {
        let skin = index
            .checked_sub(1)
            .and_then(|i| skins().into_iter().nth(i));
        skin.map_or(self.look.plain, |s| s.theme)
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
            let y = corner.1 + pad + (row + HEADER) as u32 * line + line / 6;
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

    // строка панели под указателем и доля ширины
    fn skin_point(&self, state: &Skins) -> Option<(usize, f64)> {
        let listed = state.matches().len().saturating_sub(state.top).min(ROWS);
        let rows = listed + HEADER + 1;
        let corner = self.skins_corner(rows);
        let (width, height) = self.panel_size(COLUMNS, rows);
        let (x, y) = self.pointer.position;
        let across = (x - f64::from(corner.0)) / f64::from(width.max(1));
        let down = y - f64::from(corner.1 + self.config.ui.panel.padding);
        let inside =
            (0.0..1.0).contains(&across) && down >= 0.0 && y < f64::from(corner.1 + height);
        inside.then(|| {
            (
                (down / f64::from(self.fonts.metrics().cell_height)) as usize,
                across,
            )
        })
    }

    // строка стиля листает его: левая половина назад, правая вперёд
    fn clicked(&self, state: &mut Skins) -> Outcome {
        match self.skin_point(state) {
            Some((1, across)) => {
                state.turn(if across < 0.4 { -1 } else { 1 });
                Outcome::Keep(false)
            }
            Some((0, _)) => Outcome::Preview,
            Some((row, _)) => state.click(row - HEADER).unwrap_or(Outcome::Preview),
            None => Outcome::Cancel,
        }
    }

    /// Щелчок при открытой витрине: по строке — выбрать, мимо — закрыть.
    pub(crate) fn skins_click(&mut self) -> bool {
        if !matches!(self.mode, Mode::Skins(_)) {
            return false;
        }
        let Mode::Skins(mut state) = std::mem::take(&mut self.mode) else {
            return false;
        };
        let outcome = self.clicked(&mut state);
        self.skin_outcome(state, outcome);
        self.render_status();
        true
    }
}
