//! Применение настроек к открытому окну.

use dimaster_config::{Config, Decorations};
use dimaster_i18n::t;
use dimaster_i18n::tr;

use dimaster_grid::SmartOptions;

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
        match settings::read_with(&self.hooks.overrides) {
            Ok(config) => {
                self.look.refresh();
                // новые настройки отменяют презентацию
                self.normal_config = None;
                self.config_error = false;
                if config != self.config {
                    log::info!("{}", t!("Конфиг перечитан"));
                    self.apply(config);
                } else {
                    // цвета те же, а файл приглашения мог переписать другое окно
                    self.sync_outside();
                }
                self.apply_style_bar();
            }
            Err(e) => {
                log::error!("{}", t!("Ошибка конфига: {error}", error = e));
                self.config_error = true;
            }
        }
        self.update_title();
    }

    pub(crate) fn apply(&mut self, config: Config) {
        let prev = self.config.clone();
        settings::set_language(&config);
        self.config = config;
        self.sync_fonts(&prev);
        self.apply_to_terminal();
        let size = self.window.inner_size();
        self.resize(size.width, size.height);
    }

    fn sync_fonts(&mut self, prev: &Config) {
        let font_changed = (
            &self.config.font_family,
            &self.config.font_dirs,
            self.config.font_size,
        ) != (&prev.font_family, &prev.font_dirs, prev.font_size);
        if font_changed || self.config.font_styles != prev.font_styles {
            self.zoom = 0.0;
            self.rebuild_fonts();
        }
        let style = (
            &self.config.line_height,
            &self.config.ligatures,
            &self.config.font_features,
        );
        if style != (&prev.line_height, &prev.ligatures, &prev.font_features) {
            self.fonts
                .set_style(self.config.line_height, self.config.ligatures);
            self.fonts.set_features(&self.config.font_features);
            self.renderer.reset_glyphs();
        }
    }

    pub(crate) fn apply_to_terminal(&mut self) {
        let cell_height = self.fonts.metrics().cell_height;
        let mut look = settings::appearance(&self.config, cell_height);
        // бар сверху сдвигает сетку вниз вместе с рамкой
        look.top = self.content_top();
        look.wallpaper = self.wallpaper();
        self.animator.look = self.config.cursor_look;
        self.sync_outside();
        self.renderer.set_appearance(look);
        self.term
            .set_default_colors(settings::default_colors(&self.config));
        self.term
            .set_default_cursor(settings::cursor_shape(self.config.cursor));
        let policy = self.policy();
        self.term.set_policy(policy);
        self.term.set_history_limit(self.config.scrollback);
        self.term.set_images_scroll(self.config.images_scroll);
        let tuning = &self.config.tuning;
        self.term
            .set_command_limits(tuning.memory_commands, tuning.memory_lines);
        let smart = &self.config.smart;
        self.term.set_smart_options(SmartOptions {
            ok: smart.ok.clone(),
            empty: smart.empty.clone(),
            errors: smart.errors,
            search_tools: smart.search_tools.clone(),
        });
    }

    // то, что живёт вне сетки: рамка композитора и файл приглашения
    fn sync_outside(&self) {
        let window = &self.config.window;
        self.window
            .set_decorations(window.decorations == Decorations::System);
        self.window.set_blur(window.blur);
        let template = self.look.style.map(|s| s.style.prompt);
        crate::prompt::sync(&self.config, self.look.painted, template);
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
        // высота своей рамки зависит от размера ячейки
        self.apply_to_terminal();
        self.renderer.reset_glyphs();
        let size = self.window.inner_size();
        self.resize(size.width, size.height);
    }

    pub(crate) fn update_title(&self) {
        let title = match self.term.title() {
            _ if self.config_error => tr("DimaSter — ошибка в init.lua"),
            "" => "DimaSter".to_owned(),
            title => title.to_owned(),
        };
        self.window.set_title(&title);
    }
}
