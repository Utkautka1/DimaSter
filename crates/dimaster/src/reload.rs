//! Применение настроек к открытому окну.

use dimaster_config::Config;

use crate::settings;
use crate::view::View;

// редактор мог только что обрезать файл и ещё не дописать его
fn being_written() -> bool {
    settings::config_path()
        .and_then(|p| std::fs::metadata(p).ok())
        .is_some_and(|m| m.len() == 0)
}

impl View {
    /// Перечитывает конфиг. При ошибке остаются прежние настройки.
    pub(crate) fn reload(&mut self) {
        if being_written() {
            return;
        }
        match settings::read() {
            Ok(config) => {
                self.config_error = false;
                if config != self.config {
                    log::info!("Конфиг перечитан");
                    self.apply(config);
                }
            }
            Err(e) => {
                log::error!("Ошибка конфига: {e}");
                self.config_error = true;
            }
        }
        self.update_title();
    }

    fn apply(&mut self, config: Config) {
        let font_changed = (&config.font_family, &config.font_dirs, config.font_size)
            != (
                &self.config.font_family,
                &self.config.font_dirs,
                self.config.font_size,
            );
        self.config = config;
        if font_changed {
            self.zoom = 0.0;
            self.rebuild_fonts();
        }
        self.apply_to_terminal();
        let size = self.window.inner_size();
        self.resize(size.width, size.height);
    }

    pub(crate) fn apply_to_terminal(&mut self) {
        self.renderer
            .set_appearance(settings::appearance(&self.config));
        self.term
            .set_default_colors(settings::default_colors(&self.config));
        self.term
            .set_default_cursor(settings::cursor_shape(self.config.cursor));
        self.term.set_history_limit(self.config.scrollback);
    }

    fn rebuild_fonts(&mut self) {
        match settings::fonts(&self.config, self.window.scale_factor()) {
            Ok(fonts) => {
                self.fonts = fonts;
                self.renderer.reset_glyphs();
            }
            Err(e) => log::error!("{e}"),
        }
    }

    /// `step` в пунктах; `None` — вернуть кегль из конфига.
    pub(crate) fn zoom_font(&mut self, step: Option<f32>) {
        let base = self.config.font_size;
        self.zoom = step.map_or(0.0, |s| (self.zoom + s).clamp(6.0 - base, 72.0 - base));
        self.rescale(self.window.scale_factor());
    }

    pub(crate) fn rescale(&mut self, scale: f64) {
        let points = self.config.font_size + self.zoom;
        self.fonts.set_size(settings::font_pixels(points, scale));
        self.renderer.reset_glyphs();
        let size = self.window.inner_size();
        self.resize(size.width, size.height);
    }

    pub(crate) fn update_title(&self) {
        let title = match self.term.title() {
            _ if self.config_error => "DimaSter — ошибка в init.lua",
            "" => "DimaSter",
            title => title,
        };
        self.window.set_title(title);
    }
}
