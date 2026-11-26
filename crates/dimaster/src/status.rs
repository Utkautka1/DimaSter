//! Строка статуса и монитор ресурсов в окне.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use dimaster_config::{Segment, StatusLine};
use dimaster_render::{Label, Rect};
use unicode_width::UnicodeWidthStr;

use crate::modes::Mode;
use crate::sensors::{self, Facts, Target};
use crate::view::View;

/// Последние данные и готовая строка.
#[derive(Debug, Default)]
pub(crate) struct Status {
    pub(crate) target: Arc<Mutex<Target>>,
    pub(crate) facts: Facts,
    pub(crate) line: StatusLine,
}

// полукруги Powerline из Nerd Font
const LEFT_CAP: &str = "\u{e0b6}";
const RIGHT_CAP: &str = "\u{e0b4}";

fn cells(segment: &Segment) -> usize {
    let caps = if segment.rounded && segment.background.is_some() {
        2
    } else {
        0
    };
    segment.text.width() + caps
}

fn width(segments: &[Segment]) -> usize {
    segments.iter().map(cells).sum()
}

fn mode_name(mode: &Mode) -> &'static str {
    match mode {
        Mode::Normal => "NORMAL",
        Mode::Search(_) => "ПОИСК",
        Mode::Vi(_) => "VI",
        Mode::Palette(_) => "КОМАНДЫ",
        Mode::Hint(_) => "МЕТКИ",
        Mode::History(_) => "ИСТОРИЯ",
        Mode::Minimap(_) => "КАРТА",
    }
}

impl View {
    /// Высота строки статуса в пикселях, 0 — она выключена.
    pub(crate) fn status_height(&self) -> u32 {
        crate::settings::status_height(&self.config, self.fonts.metrics().cell_height)
    }

    /// Запускает опрос данных; результаты приходят через `deliver`.
    pub(crate) fn start_sensors(&self, deliver: impl Fn(Facts) -> bool + Send + 'static) {
        let interval = Duration::from_secs_f32(self.config.widgets.interval);
        sensors::start(interval, self.status.target.clone(), deliver);
    }

    fn update_target(&self) {
        let widgets = self.config.widgets;
        let wanted = widgets.statusbar || widgets.monitor;
        let target = Target {
            cwd: wanted.then(|| self.shell_directory()).flatten(),
            shell: self
                .session
                .as_ref()
                .filter(|_| wanted)
                .map_or(0, |s| s.pid()),
            docker: widgets.statusbar && widgets.docker,
            git_changes: widgets.git_changes,
        };
        if let Ok(mut shared) = self.status.target.lock() {
            *shared = target;
        }
    }

    pub(crate) fn receive_facts(&mut self, facts: Facts) {
        self.update_target();
        self.status.facts = facts;
        self.render_status();
        let widgets = self.config.widgets;
        if widgets.statusbar || widgets.monitor {
            self.window.request_redraw();
        }
    }

    /// Строит строку заново: при новых данных и смене режима.
    pub(crate) fn render_status(&mut self) {
        if !self.config.widgets.statusbar {
            return;
        }
        let mut facts = self.status.facts.clone();
        let host = std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
        facts.push(("host".into(), host.trim().to_owned()));
        facts.push(("user".into(), std::env::var("USER").unwrap_or_default()));
        facts.push(("mode".into(), mode_name(&self.mode).into()));
        let snapshot = self.plugin_snapshot();
        let line = self
            .plugins
            .as_ref()
            .and_then(|p| p.status(&facts, snapshot));
        self.status.line = line.unwrap_or_default();
    }

    fn cell_label(&self, column: usize, y: u32, width: usize, text: &str) -> Label {
        let cell = self.fonts.metrics().cell_width;
        self.label(column as u32 * cell, y, width, text.to_owned())
    }

    // один кусок строки: подложка и, если просили, скруглённые края
    fn pill(&self, segment: &Segment, column: usize, y: u32) -> Vec<Label> {
        let color = segment.color.unwrap_or(self.config.theme.foreground);
        let Some(background) = segment.background else {
            return vec![Label {
                color,
                ..self.cell_label(column, y, 0, &segment.text)
            }];
        };
        let caps = usize::from(segment.rounded);
        let width = segment.text.width();
        let body = self.cell_label(column + caps, y, width, &segment.text);
        let mut labels = vec![Label {
            color,
            background,
            ..body
        }];
        if segment.rounded {
            for (at, text) in [(column, LEFT_CAP), (column + width + 1, RIGHT_CAP)] {
                labels.push(Label {
                    color: background,
                    ..self.cell_label(at, y, 0, text)
                });
            }
        }
        labels
    }

    fn place(&self, segments: &[Segment], mut column: usize, y: u32) -> Vec<Label> {
        let mut labels = Vec::new();
        for segment in segments {
            labels.extend(self.pill(segment, column, y));
            column += cells(segment);
        }
        labels
    }

    pub(crate) fn status_labels(&self) -> Vec<Label> {
        if self.status_height() == 0 {
            return Vec::new();
        }
        let metrics = self.fonts.metrics();
        let size = self.window.inner_size();
        let columns = (size.width / metrics.cell_width) as usize;
        let y = size.height.saturating_sub(metrics.cell_height);
        let line = &self.status.line;
        let right = columns.saturating_sub(width(&line.right));
        // центр уступает место краям
        let center = (columns.saturating_sub(width(&line.center)) / 2).max(width(&line.left) + 1);
        let mut labels = self.place(&line.left, 0, y);
        if center + width(&line.center) < right {
            labels.extend(self.place(&line.center, center, y));
        }
        labels.extend(self.place(&line.right, right, y));
        labels
    }

    pub(crate) fn status_background(&self) -> Option<Rect> {
        let height = self.status_height();
        let size = self.window.inner_size();
        (height > 0).then(|| Rect {
            x: 0,
            y: size.height.saturating_sub(height),
            width: size.width,
            height,
            color: self.config.theme.selection,
            translucent: true,
        })
    }

    pub(crate) fn monitor_labels(&self) -> Vec<Label> {
        if !self.config.widgets.monitor {
            return Vec::new();
        }
        let fact = |name: &str| self.status.facts.iter().find(|f| f.0 == name).map(|f| &f.1);
        let text = match (fact("process"), fact("cpu"), fact("memory")) {
            (Some(name), Some(cpu), Some(memory)) => format!(" {name}  CPU {cpu}%  {memory} "),
            _ => " ожидание ".to_owned(),
        };
        let metrics = self.fonts.metrics();
        let cells = text.width();
        // миникарта занимает правый край — монитор встаёт левее неё
        let reserved = if self.config.widgets.minimap { 110 } else { 0 };
        let room = self.window.inner_size().width.saturating_sub(reserved);
        let x = room.saturating_sub((cells as u32 + 1) * metrics.cell_width);
        vec![self.label(x, self.title_height() + 4, cells, text)]
    }
}
