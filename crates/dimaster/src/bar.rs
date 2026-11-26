//! Раскладка строки статуса: где подложка, строки и их куски.
//!
//! Всё, что задаётся пикселями, берётся из `statusbar.lua`; ширина текста
//! считается в ячейках шрифта терминала.

use dimaster_config::{Segment, StatusBar, StatusLine};
use dimaster_render::{Label, Rect};
use unicode_width::UnicodeWidthStr;

use crate::overlay::Drawn;
use crate::view::View;

fn cells(segment: &Segment) -> usize {
    let caps = match segment.background {
        Some(_) => segment.left_cap.width() + segment.right_cap.width(),
        None => 0,
    };
    segment.text.width() + caps
}

fn width(segments: &[Segment]) -> usize {
    segments.iter().map(cells).sum()
}

/// Высота бара вместе с отступами от краёв.
pub(crate) fn height(bar: &StatusBar, cell_height: u32) -> u32 {
    let rows = bar.rows.len() as u32;
    let style = bar.style;
    let inner = rows * cell_height + rows.saturating_sub(1) * style.gap + style.padding.1 * 2;
    inner + style.margin.1 * 2
}

/// Колонки начала левой, центральной и правой частей; центр пропадает,
/// если ему не хватает места.
fn columns(line: &StatusLine, total: usize) -> (usize, Option<usize>, usize) {
    let right = total.saturating_sub(width(&line.right));
    let center = (total.saturating_sub(width(&line.center)) / 2).max(width(&line.left) + 1);
    let fits = center + width(&line.center) < right;
    (0, fits.then_some(center), right)
}

/// Место бара в окне.
#[derive(Debug, Clone, Copy)]
struct Place {
    // левый верхний угол содержимого
    x: u32,
    y: u32,
    columns: usize,
}

impl View {
    fn bar_box(&self) -> Rect {
        let style = self.status.bar.style;
        let size = self.window.inner_size();
        let total = height(&self.status.bar, self.fonts.metrics().cell_height);
        let top = match style.top {
            true => self.title_height(),
            false => size.height.saturating_sub(total),
        };
        Rect {
            x: style.margin.0,
            y: top + style.margin.1,
            width: size.width.saturating_sub(style.margin.0 * 2),
            height: total.saturating_sub(style.margin.1 * 2),
            color: style.background.unwrap_or(self.config.theme.background),
            translucent: false,
            radius: style.radius,
            opacity: style.opacity,
        }
    }

    fn text_label(&self, place: Place, column: usize, text: &str) -> Label {
        Label {
            x: place.x + column as u32 * self.fonts.metrics().cell_width,
            y: place.y,
            width: 0,
            text: text.to_owned(),
            color: self.config.theme.foreground,
            background: self.config.theme.background,
        }
    }

    // подложка куска: прямоугольник со скруглением и символы по краям
    fn chip(&self, place: Place, segment: &Segment, column: usize, out: &mut Drawn) {
        let Some(background) = segment.background else {
            return;
        };
        let metrics = self.fonts.metrics();
        let (left, body) = (segment.left_cap.width(), segment.text.width());
        out.rects.push(Rect {
            x: place.x + (column + left) as u32 * metrics.cell_width,
            y: place.y,
            width: body as u32 * metrics.cell_width,
            height: metrics.cell_height,
            color: background,
            translucent: false,
            radius: segment.radius,
            opacity: segment.opacity,
        });
        let caps = [
            (column, &segment.left_cap),
            (column + left + body, &segment.right_cap),
        ];
        for (at, cap) in caps.into_iter().filter(|c| !c.1.is_empty()) {
            out.labels.push(Label {
                color: background,
                ..self.text_label(place, at, cap)
            });
        }
    }

    fn side(&self, place: Place, segments: &[Segment], mut column: usize, out: &mut Drawn) {
        for segment in segments {
            self.chip(place, segment, column, out);
            let color = segment.color.unwrap_or(self.config.theme.foreground);
            let caps = segment.background.map_or(0, |_| segment.left_cap.width());
            let label = self.text_label(place, column + caps, &segment.text);
            out.labels.push(Label { color, ..label });
            column += cells(segment);
        }
    }

    fn row(&self, place: Place, line: &StatusLine, out: &mut Drawn) {
        let (left, center, right) = columns(line, place.columns);
        self.side(place, &line.left, left, out);
        if let Some(center) = center {
            self.side(place, &line.center, center, out);
        }
        self.side(place, &line.right, right, out);
    }

    pub(crate) fn status_bar(&self) -> Drawn {
        let mut out = Drawn::default();
        if self.status_height() == 0 {
            return out;
        }
        let style = self.status.bar.style;
        let metrics = self.fonts.metrics();
        let area = self.bar_box();
        if style.background.is_some() {
            out.rects.push(area);
        }
        let inner = area.width.saturating_sub(style.padding.0 * 2);
        let columns = (inner / metrics.cell_width) as usize;
        for (index, line) in self.status.bar.rows.iter().enumerate() {
            let y = area.y + style.padding.1 + index as u32 * (metrics.cell_height + style.gap);
            self.row(
                Place {
                    x: area.x + style.padding.0,
                    y,
                    columns,
                },
                line,
                &mut out,
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(text: &str, cap: &str) -> Segment {
        Segment {
            text: text.into(),
            color: None,
            background: (!cap.is_empty()).then_some(1),
            left_cap: cap.into(),
            right_cap: cap.into(),
            radius: 0,
            opacity: 1.0,
        }
    }

    #[test]
    fn rows_padding_and_margins_add_up() {
        let mut bar = StatusBar::default();
        assert_eq!(height(&bar, 20), 20 + 2 * 10);
        bar.rows.push(StatusLine::default());
        bar.style.padding = (0, 5);
        assert_eq!(height(&bar, 20), 40 + 4 + 10 + 20);
    }

    #[test]
    fn center_yields_to_sides() {
        let mut line = StatusLine {
            left: vec![segment("abc", "<")],
            center: vec![segment("mid", "")],
            right: vec![segment("r", "")],
        };
        assert_eq!(columns(&line, 21), (0, Some(9), 20));
        line.center = vec![segment("very long centre text", "")];
        assert_eq!(columns(&line, 21).1, None);
    }
}
