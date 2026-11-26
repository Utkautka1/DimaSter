//! Строка статуса и монитор ресурсов в окне.

use dimaster_i18n::tr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dimaster_config::StatusBar;
use unicode_width::UnicodeWidthStr;

use crate::bar;
use crate::modes::Mode;
use crate::overlay::{Drawn, fill};
use crate::sensors::{self, Facts, Target};
use crate::view::View;

/// Последние данные и готовая строка.
#[derive(Debug, Default)]
pub(crate) struct Status {
    pub(crate) target: Arc<Mutex<Target>>,
    pub(crate) facts: Facts,
    pub(crate) bar: StatusBar,
}

fn mode_name(mode: &Mode) -> String {
    let name = match mode {
        Mode::Normal => "NORMAL",
        Mode::Search(_) => "ПОИСК",
        Mode::Vi(_) => "VI",
        Mode::Palette(_) => "КОМАНДЫ",
        Mode::Hint(_) => "МЕТКИ",
        Mode::History(_) => "ИСТОРИЯ",
        Mode::Minimap(_) => "КАРТА",
    };
    tr(name)
}

impl View {
    /// Высота бара в пикселях, 0 — он выключен.
    pub(crate) fn status_height(&self) -> u32 {
        match self.config.widgets.statusbar {
            true => bar::height(&self.status.bar, self.fonts.metrics().cell_height),
            false => 0,
        }
    }

    /// Где начинается сетка терминала по вертикали.
    pub(crate) fn content_top(&self) -> u32 {
        let top_bar = if self.status.bar.style.top {
            self.status_height()
        } else {
            0
        };
        self.title_height() + top_bar
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

    /// Строит бар заново: при новых данных и смене режима.
    pub(crate) fn render_status(&mut self) {
        if !self.config.widgets.statusbar {
            return;
        }
        let mut facts = self.status.facts.clone();
        let host = std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
        facts.push(("host".into(), host.trim().to_owned()));
        facts.push(("user".into(), std::env::var("USER").unwrap_or_default()));
        facts.push(("mode".into(), mode_name(&self.mode)));
        let snapshot = self.plugin_snapshot();
        let bar = self
            .plugins
            .as_ref()
            .and_then(|p| p.status(&facts, snapshot));
        self.replace_bar(bar.unwrap_or_default());
    }

    // другая высота или сторона меняют размер сетки терминала
    fn replace_bar(&mut self, bar: StatusBar) {
        let before = (self.status_height(), self.content_top());
        self.status.bar = bar;
        if before != (self.status_height(), self.content_top()) {
            self.apply_to_terminal();
            let size = self.window.inner_size();
            self.resize(size.width, size.height);
        }
    }

    fn monitor_text(&self) -> String {
        let style = &self.config.ui.monitor;
        let fact = |name: &str| self.status.facts.iter().find(|f| f.0 == name).map(|f| &f.1);
        match (fact("process"), fact("cpu"), fact("memory")) {
            (Some(process), Some(cpu), Some(memory)) => fill(
                &tr(&style.format),
                &[("process", process), ("cpu", cpu), ("memory", memory)],
            ),
            _ => tr(&style.waiting),
        }
    }

    pub(crate) fn monitor_panel(&self) -> Drawn {
        // история занимает тот же край — подписи наложились бы друг на друга
        if !self.config.widgets.monitor || matches!(self.mode, Mode::History(_)) {
            return Drawn::default();
        }
        let text = self.monitor_text();
        let columns = text.width();
        self.panel(self.monitor_corner(columns), columns, &[(text, false)])
    }

    fn monitor_corner(&self, columns: usize) -> (u32, u32) {
        let (style, minimap) = (&self.config.ui.monitor, self.config.ui.minimap);
        let size = self.window.inner_size();
        let (width, height) = self.panel_size(columns, 1);
        let cell = self.fonts.metrics().cell_width;
        // миникарта у того же края — монитор встаёт рядом с ней
        let beside = self.config.widgets.minimap && minimap.left == style.left;
        let inset = cell + if beside { minimap.width + 20 } else { 0 };
        let x = match style.left {
            true => inset,
            false => size.width.saturating_sub(width + inset),
        };
        let y = match style.bottom {
            true => size
                .height
                .saturating_sub(height + self.content_bottom() + 4),
            false => self.content_top() + 4,
        };
        (x, y)
    }
}
