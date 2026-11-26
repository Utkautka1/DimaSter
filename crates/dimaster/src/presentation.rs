//! Режим презентации: крупный шрифт, поля, без рамки и полосы прокрутки.

use dimaster_config::{Config, Decorations};

use crate::view::View;

// на сколько пунктов крупнее
const ZOOM: f32 = 4.0;

/// Настройки для показа на экране на основе обычных.
pub(crate) fn derive(config: &Config, window_width: u32) -> Config {
    let mut shown = config.clone();
    shown.font_size += ZOOM;
    shown.padding = (window_width / 10, config.padding.1.max(24));
    shown.window.decorations = Decorations::None;
    shown.scrollbar = false;
    shown.effects.cursor_trail = false;
    shown
}

impl View {
    pub(crate) fn toggle_presentation(&mut self) {
        let next = match self.normal_config.take() {
            Some(normal) => normal,
            None => {
                self.normal_config = Some(self.config.clone());
                derive(&self.config, self.window.inner_size().width)
            }
        };
        self.apply(next);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_chrome_and_enlarges_font() {
        let shown = derive(&Config::default(), 1000);
        assert_eq!((shown.font_size, shown.padding.0), (16.0, 100));
        assert!(!shown.scrollbar);
        assert_eq!(shown.window.decorations, Decorations::None);
    }
}
