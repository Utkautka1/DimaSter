//! Сборка кадра: подсветки, панели и положение курсора.

use dimaster_i18n::{t, tr};
use std::time::Instant;

use dimaster_config::Anchor;
use dimaster_grid::{Point, Span};
use dimaster_render::Viewport;
use unicode_width::UnicodeWidthStr;

use crate::modes::Mode;
use crate::overlay::{Drawn, fill};
use crate::view::View;

impl View {
    fn highlights(&self) -> Vec<(Span, u32)> {
        let (marks, mut out) = (self.config.ui.marks, Vec::new());
        if let Mode::Search(search) = &self.mode {
            let first = self.term.visible_line(self.scroll);
            let last = first + self.term.screen().rows().len() as u64;
            for (index, found) in search.matches.iter().enumerate() {
                if found.end.line >= first && found.start.line < last {
                    let current = index == search.current;
                    let color = if current { marks.current } else { marks.found };
                    out.push((self.term.match_span(*found), color));
                }
            }
        }
        out.extend(
            self.hover_link
                .map(|l| (self.term.match_span(l), marks.link)),
        );
        out
    }

    fn syntax_highlights(&mut self) -> Vec<(Span, u32)> {
        if !self.config.smart.syntax_highlight || !matches!(self.mode, Mode::Normal) {
            return Vec::new();
        }
        crate::syntax::highlight_input(&self.term, &self.config.theme.ansi, &mut self.syntax)
    }

    /// Возвращает текст призрачной подсказки или исправления опечатки для отрисовки.
    pub(crate) fn ghost_suggestion(&mut self) -> Option<String> {
        if !self.config.smart.autosuggest
            || !matches!(self.mode, Mode::Normal)
            || self.term.is_alternate()
            || !self.preedit.is_empty()
        {
            return None;
        }
        let prefix = crate::syntax::input_text_at_end(&self.term)?;
        let live: Vec<String> = self.term.commands().map(|c| c.command.clone()).collect();
        if let Some(tail) = self.history_store.best_suggestion(&prefix, &live) {
            return Some(tail);
        }
        crate::syntax::suggest_typo(&prefix, self.term.cwd(), &mut self.syntax)
            .map(|(_, fix)| self.config.help.typo.replace("{fix}", &fix))
            .filter(|hint| !hint.is_empty())
    }

    /// Сколько пикселей снизу занято баром.
    pub(crate) fn content_bottom(&self) -> u32 {
        match self.status.bar.style.top {
            true => 0,
            false => self.status_height(),
        }
    }

    fn columns(&self) -> usize {
        (self.window.inner_size().width / self.fonts.metrics().cell_width) as usize
    }

    // строка у края: поиск, подсказка режима или адрес ссылки
    fn edge_line(&self, columns: usize, text: String) -> Drawn {
        let height = self.panel_size(columns, 1).1;
        let y = match self.config.ui.prompt_top {
            true => self.content_top(),
            false => self
                .window
                .inner_size()
                .height
                .saturating_sub(height + self.content_bottom()),
        };
        self.panel((0, y), columns, &[(text, false)])
    }

    fn search_text(&self, search: &crate::modes::search::Search) -> String {
        let texts = &self.config.ui.texts;
        let kind = tr(if search.regex {
            &texts.search_regex
        } else {
            &texts.search_plain
        });
        let position = if search.matches.is_empty() {
            0
        } else {
            search.current + 1
        };
        let values = [
            ("query", search.query.as_str()),
            ("position", &position.to_string()),
            ("count", &search.matches.len().to_string()),
            ("kind", &kind),
        ];
        fill(&tr(&texts.search), &values)
    }

    fn prompt(&self) -> Drawn {
        let texts = &self.config.ui.texts;
        let text = match &self.mode {
            Mode::Search(search) => self.search_text(search),
            Mode::Hint(_) => tr(&texts.hint),
            Mode::Minimap(_) => tr(&texts.minimap),
            Mode::Vi(_) => tr(&texts.vi),
            Mode::Paste(paste) => t!(
                " Опасная вставка ({reason}): Enter — вставить · Пробел — в одну строку · Esc — отмена ",
                reason = paste.reason
            ),
            Mode::CloseConfirm => t!(" Закрыть окно терминала? Enter/y — да · Esc/n — отмена "),
            _ => return Drawn::default(),
        };
        self.edge_line(self.columns(), text)
    }

    fn palette_panel(&self) -> Drawn {
        let Mode::Palette(palette) = &self.mode else {
            return Drawn::default();
        };
        let style = &self.config.ui.palette;
        let columns = self.columns().min(style.width);
        let mut lines = vec![(format!("{}{}_", style.prompt, palette.query), false)];
        // список листается, чтобы выбранная команда оставалась видна
        let start = palette.selected.saturating_sub(style.rows - 1);
        let items = palette
            .items()
            .into_iter()
            .enumerate()
            .skip(start)
            .take(style.rows);
        lines.extend(items.map(|(i, (name, _))| (format!("   {name}"), i == palette.selected)));
        self.panel(self.palette_corner(columns, lines.len()), columns, &lines)
    }

    fn palette_corner(&self, columns: usize, rows: usize) -> (u32, u32) {
        let size = self.window.inner_size();
        let (width, height) = self.panel_size(columns, rows);
        let cell = self.fonts.metrics().cell_height;
        let y = match self.config.ui.palette.position {
            Anchor::Top => self.title_height() + cell,
            Anchor::Center => size.height.saturating_sub(height) / 2,
            Anchor::Bottom => size
                .height
                .saturating_sub(height + self.content_bottom() + cell),
        };
        (size.width.saturating_sub(width) / 2, y)
    }

    // (строка, колонка) на экране; None — курсор за пределами вида
    fn screen_position(&self, point: Point) -> Option<(f32, f32)> {
        let row = point
            .line
            .checked_sub(self.term.visible_line(self.scroll))?;
        (row < self.term.screen().rows().len() as u64).then_some((row as f32, point.col as f32))
    }

    fn cursor_target(&self) -> Option<(f32, f32)> {
        if let Mode::Vi(vi) = &self.mode {
            return self.screen_position(vi.cursor);
        }
        let cursor = self.term.cursor();
        let visible = self.scroll == 0 && self.term.modes().cursor_visible;
        visible.then_some((cursor.row as f32, cursor.col as f32))
    }

    // адрес ссылки под указателем при зажатом Ctrl
    fn link_panel(&self) -> Drawn {
        let Some(uri) = self.hover_link.and(self.hover_uri.as_ref()) else {
            return Drawn::default();
        };
        let columns = (uri.width() + 2).min(self.columns());
        self.edge_line(columns, format!(" {uri}"))
    }

    // интерфейс снизу вверх по слоям
    fn interface(&self) -> Drawn {
        let mut out = Drawn {
            rects: self.minimap_rects(),
            ..Drawn::default()
        };
        out.merge(self.canvas_layer());
        out.merge(self.title_bar());
        out.merge(self.status_bar());
        out.rects.extend(self.scrollbar_rects());
        out.merge(self.prompt());
        out.merge(self.palette_panel());
        out.merge(self.mode_panels());
        out.merge(self.skins_panel());
        out.merge(self.link_panel());
        out.merge(self.monitor_panel());
        out.merge(self.notice_panel());
        out.rects.extend(self.glow_rects());
        out.rects.extend(self.signal_rects());
        out
    }

    fn selection_span(&self) -> Option<Span> {
        let selection = match &self.mode {
            Mode::Vi(vi) => vi.selection(),
            _ => self.selecting.selection,
        };
        selection.map(|s| self.term.span(s))
    }

    fn tick_cursor(&mut self) -> (Option<(f32, f32)>, crate::animate::Pose) {
        let target = self.cursor_target();
        let effects = self.config.effects;
        let at = target.unwrap_or_default();
        let pose = self
            .animator
            .tick(at, effects, self.focused, Instant::now());
        (target, pose)
    }

    pub(crate) fn draw(&mut self) {
        let (target, pose) = self.tick_cursor();
        let (selection, highlights) = (self.selection_span(), self.highlights());
        let (syntax, ghost) = (self.syntax_highlights(), self.ghost_suggestion());
        let Drawn { rects, labels } = self.interface();
        let trail: &[_] = if target.is_some() { &pose.trail } else { &[] };
        let view = Viewport {
            scroll: self.scroll,
            selection,
            preedit: &self.preedit,
            ghost: ghost.as_deref(),
            highlights: &highlights,
            syntax: &syntax,
            labels: &labels,
            offset: pose.offset,
            cursor: target.map(|_| pose.cursor),
            cursor_hidden: target.is_none() || pose.blink_hidden,
            unfocused: !self.focused,
            shrink: pose.shrink,
            trail,
            rects: &rects,
        };
        self.renderer.prepare(&self.term, &mut self.fonts, view);
        self.show();
    }

    fn show(&mut self) {
        // показ кадра может ждать монитор — вывод тем временем разбирается
        self.term.release();
        if !self.renderer.present() {
            (self.wake)();
        }
        self.schedule_animation();
    }
}
