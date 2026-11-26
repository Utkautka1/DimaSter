//! Общие кирпичи интерфейса поверх терминала: панели и их строки.
//!
//! Подложки (`rects`) рисуются под всеми подписями, поэтому панели не должны
//! накладываться друг на друга: сквозь верхнюю проступил бы текст нижней.

use dimaster_render::{Label, Rect};
use unicode_width::UnicodeWidthChar;

use crate::view::View;

#[derive(Default)]
pub(crate) struct Drawn {
    pub(crate) rects: Vec<Rect>,
    pub(crate) labels: Vec<Label>,
}

impl Drawn {
    pub(crate) fn merge(&mut self, other: Drawn) {
        self.rects.extend(other.rects);
        self.labels.extend(other.labels);
    }
}

// доля `amount` второго цвета в смеси
fn blend(base: u32, other: u32, amount: f32) -> u32 {
    let channel = |shift: u32| {
        let (a, b) = (
            ((base >> shift) & 0xff) as f32,
            ((other >> shift) & 0xff) as f32,
        );
        ((a + (b - a) * amount).round() as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

/// Начало строки, помещающееся в `columns` ячеек.
pub(crate) fn clip(text: &str, columns: usize) -> String {
    let mut used = 0;
    let fits = text.chars().take_while(|c| {
        used += c.width().unwrap_or(1);
        used <= columns
    });
    fits.collect()
}

/// Подставляет значения вместо `{имя}`.
pub(crate) fn fill(template: &str, values: &[(&str, &str)]) -> String {
    values
        .iter()
        .fold(template.to_owned(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
}

impl View {
    pub(crate) fn plain_rect(&self, x: u32, y: u32, size: (u32, u32), color: u32) -> Rect {
        let (width, height) = size;
        Rect {
            x,
            y,
            width,
            height,
            color,
            translucent: false,
            radius: 0,
            opacity: 1.0,
        }
    }

    // цвета панелей выводятся из фона и текста темы: цвет выделения бывает
    // светлым, и светлый текст на нём не читается
    fn panel_colors(&self, selected: bool) -> (u32, u32) {
        let (style, theme) = (self.config.ui.panel, self.config.theme);
        let color = style.color.unwrap_or(theme.foreground);
        let background = style
            .background
            .unwrap_or_else(|| blend(theme.background, theme.foreground, 0.14));
        match selected {
            false => (color, background),
            true => (
                style.selected_color.unwrap_or(theme.background),
                style.selected_background.unwrap_or(color),
            ),
        }
    }

    /// Размер панели в пикселях вместе с полями.
    pub(crate) fn panel_size(&self, columns: usize, rows: usize) -> (u32, u32) {
        let (metrics, pad) = (self.fonts.metrics(), self.config.ui.panel.padding * 2);
        let width = columns as u32 * metrics.cell_width + pad;
        (width, rows as u32 * metrics.cell_height + pad)
    }

    fn backdrop(&self, at: (u32, u32), size: (u32, u32)) -> Rect {
        let style = self.config.ui.panel;
        let (_, color) = self.panel_colors(false);
        let rect = self.plain_rect(at.0, at.1, size, color);
        Rect {
            radius: style.radius,
            opacity: style.opacity,
            ..rect
        }
    }

    /// Панель с левым верхним углом `at`; пара — текст строки и выбрана ли она.
    pub(crate) fn panel(&self, at: (u32, u32), columns: usize, lines: &[(String, bool)]) -> Drawn {
        let (metrics, pad) = (self.fonts.metrics(), self.config.ui.panel.padding);
        let mut out = Drawn::default();
        out.rects
            .push(self.backdrop(at, self.panel_size(columns, lines.len())));
        for (index, (text, selected)) in lines.iter().enumerate() {
            let (x, y) = (at.0 + pad, at.1 + pad + index as u32 * metrics.cell_height);
            let (color, background) = self.panel_colors(*selected);
            if *selected {
                let size = (columns as u32 * metrics.cell_width, metrics.cell_height);
                out.rects.push(self.plain_rect(x, y, size, background));
            }
            let text = clip(text, columns);
            out.labels.push(Label {
                x,
                y,
                width: 0,
                text,
                color,
                background,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blends_clips_and_fills() {
        assert_eq!(blend(0x000000, 0xffffff, 0.5), 0x808080);
        assert_eq!(clip("абв漢字", 5), "абв漢");
        assert_eq!(fill("{a}/{b} {c}", &[("a", "1"), ("b", "2")]), "1/2 {c}");
    }
}
