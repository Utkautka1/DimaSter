//! Панели новых режимов: метки подсказок и история команд.

use dimaster_config::Texts;
use dimaster_grid::{Finished, describe_exit};
use dimaster_i18n::{t, tr};
use dimaster_render::Label;

use crate::modes::picker::{Kind, Picker};
use crate::modes::{Mode, launch};
use crate::overlay::{Drawn, fill};
use crate::view::View;

// код возврата и вывод выбранной команды
fn details(entry: &Finished, texts: &Texts) -> Vec<(String, bool)> {
    let status = match entry.exit {
        0 => tr(&texts.history_output),
        code => fill(
            &tr(&texts.history_failed),
            &[
                ("code", &code.to_string()),
                ("reason", &describe_exit(code)),
            ],
        ),
    };
    let mut lines = vec![(format!(" ── {status} ──"), false)];
    lines.extend(entry.output.iter().map(|l| (format!(" {l}"), false)));
    lines
}

// что за список и как им пользоваться
fn picker_header(picker: &Picker) -> String {
    let q = &picker.query;
    match picker.kind {
        Kind::Files => t!(" Файлы: {q}_", q = q),
        Kind::Dirs => t!(" Папки: {q}_", q = q),
        Kind::Ssh => t!(" SSH: {q}_  (Enter подключиться)", q = q),
        Kind::Snippets => t!(" Сниппеты: {q}_", q = q),
        Kind::Docker => t!(
            " Docker: {q}_  (Enter войти · ^L логи · ^S старт/стоп · ^R перезапуск)",
            q = q
        ),
        Kind::Fill => {
            let template = picker
                .values
                .first()
                .map(String::as_str)
                .unwrap_or_default();
            let field = launch::next_field(template).unwrap_or_default();
            t!(" {field}: {q}_", field = field, q = q)
        }
    }
}

impl View {
    fn hint_labels(&self) -> Vec<Label> {
        let Mode::Hint(hints) = &self.mode else {
            return Vec::new();
        };
        let (metrics, marks) = (self.fonts.metrics(), self.config.ui.marks);
        let first = self.term.visible_line(self.scroll);
        let visible = hints
            .targets
            .iter()
            .filter(|t| t.label.starts_with(&hints.typed));
        visible
            .filter_map(|target| {
                let row = u32::try_from(target.at.line.checked_sub(first)?).ok()?;
                Some(Label {
                    x: self.config.padding.0 + target.at.col as u32 * metrics.cell_width,
                    y: self.content_top() + self.config.padding.1 + row * metrics.cell_height,
                    width: target.label.len(),
                    text: target.label.clone(),
                    color: marks.hint_color,
                    background: marks.hint_background,
                })
            })
            .collect()
    }

    // строки панели: поиск, список команд, затем вывод выбранной
    fn history_lines(&self) -> Vec<(String, bool)> {
        let Mode::History(history) = &self.mode else {
            return Vec::new();
        };
        let (texts, rows) = (&self.config.ui.texts, self.config.ui.history.rows);
        let filter = t!(" Поиск: {q}_", q = history.query);
        let mut lines = vec![(tr(&texts.history), false), (filter, false)];
        let filtered = history.filtered();
        let start = history.selected.saturating_sub(rows - 1);
        for (pos, (_, entry)) in filtered.iter().enumerate().skip(start).take(rows) {
            let mark = if entry.exit == 0 { '✓' } else { '✗' };
            let seconds = entry.duration.as_secs_f32();
            let text = format!(" {mark} {seconds:>5.1}{}  {}", tr("с"), entry.command);
            lines.push((text, pos == history.selected));
        }
        if let Some(entry) = history.selected_entry() {
            lines.extend(details(entry, texts));
        }
        lines
    }

    fn history_panel(&self) -> Drawn {
        let (metrics, style) = (self.fonts.metrics(), self.config.ui.history);
        let size = self.window.inner_size();
        let all = (size.width / metrics.cell_width) as usize;
        let columns = ((all as f32 * style.width) as usize)
            .max(dimaster_limits::HISTORY_COLUMNS_MIN.size())
            .min(all);
        let (top, pad) = (self.title_height(), self.config.ui.panel.padding * 2);
        let height = size
            .height
            .saturating_sub(top + self.content_bottom() + pad);
        let mut lines = self.history_lines();
        lines.truncate((height / metrics.cell_height) as usize);
        let x = match style.left {
            true => 0,
            false => size.width.saturating_sub(self.panel_size(columns, 0).0),
        };
        self.panel((x, top), columns, &lines)
    }

    fn picker_panel(&self) -> Drawn {
        let Mode::Picker(picker) = &self.mode else {
            return Drawn::default();
        };
        let style = &self.config.ui.palette;
        let columns = (self.window.inner_size().width / self.fonts.metrics().cell_width) as usize;
        let wide = if picker.kind == Kind::Docker { 80 } else { 56 };
        let columns = columns.min(style.width.max(wide));
        let header = picker_header(picker);
        let mut lines = vec![(header, false)];
        let start = picker.selected.saturating_sub(style.rows - 1);
        for (i, item) in picker
            .matches()
            .into_iter()
            .enumerate()
            .skip(start)
            .take(style.rows)
        {
            lines.push((format!("   {item}"), i == picker.selected));
        }
        let pos = self.centered_panel(columns, lines.len());
        self.panel(pos, columns, &lines)
    }

    fn centered_panel(&self, columns: usize, rows: usize) -> (u32, u32) {
        let size = self.window.inner_size();
        let (width, height) = self.panel_size(columns, rows);
        let y = self.title_height() + self.fonts.metrics().cell_height;
        let _ = height;
        (size.width.saturating_sub(width) / 2, y)
    }

    pub(crate) fn mode_panels(&self) -> Drawn {
        let mut out = Drawn {
            labels: self.hint_labels(),
            ..Drawn::default()
        };
        if matches!(self.mode, Mode::History(_)) {
            out.merge(self.history_panel());
        }
        if matches!(self.mode, Mode::Picker(_)) {
            out.merge(self.picker_panel());
        }
        out
    }
}
