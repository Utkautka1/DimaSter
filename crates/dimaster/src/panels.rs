//! Панели новых режимов: метки подсказок и история команд.

use dimaster_grid::{Finished, describe_exit};
use dimaster_render::Label;

use crate::modes::Mode;
use crate::view::View;

const HINT_TEXT: u32 = 0x11131a;
const HINT_BACKGROUND: u32 = 0xe0af68;
const COMMAND_ROWS: usize = 10;

// код возврата и вывод выбранной команды
fn details(entry: &Finished) -> Vec<(String, bool)> {
    let status = match entry.exit {
        0 => "вывод".to_owned(),
        code => format!("код {code}: {}", describe_exit(code)),
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
        let metrics = self.fonts.metrics();
        let first = self.term.visible_line(self.scroll);
        let visible = hints
            .targets
            .iter()
            .filter(|t| t.label.starts_with(&hints.typed));
        visible
            .filter_map(|target| {
                let row = u32::try_from(target.at.line.checked_sub(first)?).ok()?;
                let x = self.config.padding.0 + target.at.col as u32 * metrics.cell_width;
                let y = self.title_height() + self.config.padding.1 + row * metrics.cell_height;
                let mut label = self.label(x, y, target.label.len(), target.label.clone());
                (label.color, label.background) = (HINT_TEXT, HINT_BACKGROUND);
                Some(label)
            })
            .collect()
    }

    // строки панели: список команд, затем вывод выбранной
    fn history_lines(&self) -> Vec<(String, bool)> {
        let Mode::History(history) = &self.mode else {
            return Vec::new();
        };
        let mut lines = vec![(" j/k выбор · Enter вставить · y вывод · q".into(), false)];
        let start = history.selected.saturating_sub(COMMAND_ROWS - 1);
        let entries = history
            .entries
            .iter()
            .enumerate()
            .skip(start)
            .take(COMMAND_ROWS);
        for (index, entry) in entries {
            let mark = if entry.exit == 0 { '✓' } else { '✗' };
            let seconds = entry.duration.as_secs_f32();
            let text = format!(" {mark} {seconds:>5.1}с  {}", entry.command);
            lines.push((text, index == history.selected));
        }
        if let Some(entry) = history.entries.get(history.selected) {
            lines.extend(details(entry));
        }
        lines
    }

    fn history_labels(&self) -> Vec<Label> {
        let metrics = self.fonts.metrics();
        let size = self.window.inner_size();
        let columns = (size.width / metrics.cell_width) as usize;
        let width = (columns * 3 / 5).max(30).min(columns);
        let x = (columns - width) as u32 * metrics.cell_width;
        let top = self.title_height();
        let room = ((size.height - top) / metrics.cell_height) as usize;
        let rows = self.history_lines().into_iter().take(room).enumerate();
        rows.map(|(index, (text, selected))| {
            let y = top + index as u32 * metrics.cell_height;
            let mut label = self.label(x, y, width, text);
            if selected {
                (label.background, label.color) = (label.color, self.config.theme.background);
            }
            label
        })
        .collect()
    }

    pub(crate) fn mode_panels(&self) -> Vec<Label> {
        let mut labels = self.hint_labels();
        if matches!(self.mode, Mode::History(_)) {
            labels.extend(self.history_labels());
        }
        labels
    }
}
