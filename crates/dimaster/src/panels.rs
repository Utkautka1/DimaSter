//! Панели новых режимов: метки подсказок и история команд.

use dimaster_config::Texts;
use dimaster_grid::{Finished, describe_exit};
use dimaster_i18n::tr;
use dimaster_render::Label;

use crate::modes::Mode;
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

    // строки панели: список команд, затем вывод выбранной
    fn history_lines(&self) -> Vec<(String, bool)> {
        let Mode::History(history) = &self.mode else {
            return Vec::new();
        };
        let (texts, rows) = (&self.config.ui.texts, self.config.ui.history.rows);
        let mut lines = vec![(tr(&texts.history), false)];
        let start = history.selected.saturating_sub(rows - 1);
        let entries = history.entries.iter().enumerate().skip(start).take(rows);
        for (index, entry) in entries {
            let mark = if entry.exit == 0 { '✓' } else { '✗' };
            let seconds = entry.duration.as_secs_f32();
            let unit = tr("с");
            let text = format!(" {mark} {seconds:>5.1}{unit}  {}", entry.command);
            lines.push((text, index == history.selected));
        }
        if let Some(entry) = history.entries.get(history.selected) {
            lines.extend(details(entry, texts));
        }
        lines
    }

    fn history_panel(&self) -> Drawn {
        let (metrics, style) = (self.fonts.metrics(), self.config.ui.history);
        let size = self.window.inner_size();
        let all = (size.width / metrics.cell_width) as usize;
        let columns = ((all as f32 * style.width) as usize).max(30).min(all);
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

    pub(crate) fn mode_panels(&self) -> Drawn {
        let mut out = Drawn {
            labels: self.hint_labels(),
            ..Drawn::default()
        };
        if matches!(self.mode, Mode::History(_)) {
            out.merge(self.history_panel());
        }
        out
    }
}
