//! Фигуры из `draw.lua` в прямоугольники и подписи окна.

use dimaster_config::{Paint, Shape};
use dimaster_render::{Label, Rect, Stroke};

use crate::overlay::Drawn;
use crate::view::View;

// многоугольник выше этого окна всё равно не бывает
const MAX_SPANS: i32 = 4000;

/// Отрицательная координата отсчитывается от дальнего края.
fn place(value: f32, size: u32) -> f32 {
    match value < 0.0 {
        true => (size as f32 + value).max(0.0),
        false => value,
    }
}

// пересечения горизонтали `y` с рёбрами, по возрастанию
fn crossings(points: &[(f32, f32)], y: f32) -> Vec<f32> {
    let edges = points.iter().zip(points.iter().cycle().skip(1));
    let mut found: Vec<f32> = edges
        .filter(|(a, b)| (a.1 <= y) != (b.1 <= y))
        .map(|(a, b)| a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0))
        .collect();
    found.sort_by(f32::total_cmp);
    found
}

/// Заливка по строкам в один пиксель: (x, y, ширина), правило чёт-нечет.
fn spans(points: &[(f32, f32)]) -> Vec<(u32, u32, u32)> {
    let top = points.iter().map(|p| p.1).fold(f32::MAX, f32::min);
    let bottom = points.iter().map(|p| p.1).fold(f32::MIN, f32::max);
    let (first, last) = (top.max(0.0) as i32, bottom.ceil() as i32);
    let mut out = Vec::new();
    for y in first..last.min(first + MAX_SPANS) {
        for pair in crossings(points, y as f32 + 0.5).as_chunks::<2>().0 {
            let (left, right) = (pair[0].max(0.0).round(), pair[1].max(0.0).round());
            if right > left {
                out.push((left as u32, y as u32, (right - left) as u32));
            }
        }
    }
    out
}

// прямоугольник вокруг отрезка с полем в половину толщины
fn segment(from: (f32, f32), to: (f32, f32), width: f32) -> (f32, f32, f32, f32, Stroke) {
    let half = width / 2.0;
    let x = (from.0.min(to.0) - half).max(0.0);
    let y = (from.1.min(to.1) - half).max(0.0);
    let size = ((from.0 - to.0).abs() + width, (from.1 - to.1).abs() + width);
    let rising = (to.0 - from.0) * (to.1 - from.1) < 0.0;
    (x, y, size.0, size.1, Stroke { width, rising })
}

impl View {
    fn painted(&self, area: (f32, f32, f32, f32), paint: Paint) -> Rect {
        let (x, y, width, height) = area;
        let color = paint.color.unwrap_or(self.config.theme.foreground);
        let size = (
            width.max(0.0).round() as u32,
            height.max(0.0).round() as u32,
        );
        let rect = self.plain_rect(x.round() as u32, y.round() as u32, size, color);
        Rect {
            opacity: paint.opacity,
            ..rect
        }
    }

    fn on_screen(&self, (x, y): (f32, f32)) -> (f32, f32) {
        let size = self.window.inner_size();
        (place(x, size.width), place(y, size.height))
    }

    fn polygon(&self, points: &[(f32, f32)], paint: Paint, out: &mut Drawn) {
        let points: Vec<_> = points.iter().map(|&p| self.on_screen(p)).collect();
        for (x, y, width) in spans(&points) {
            let area = (x as f32, y as f32, width as f32, 1.0);
            out.rects.push(self.painted(area, paint));
        }
    }

    fn line(&self, from: (f32, f32), to: (f32, f32), width: f32, paint: Paint) -> Rect {
        let (x, y, w, h, stroke) = segment(self.on_screen(from), self.on_screen(to), width);
        Rect {
            stroke: Some(stroke),
            ..self.painted((x, y, w, h), paint)
        }
    }

    fn text_shape(&self, at: (f32, f32), text: &str, paint: Paint) -> Label {
        let (x, y) = self.on_screen(at);
        Label {
            x: x as u32,
            y: y as u32,
            width: 0,
            text: text.chars().take(500).collect(),
            color: paint.color.unwrap_or(self.config.theme.foreground),
            background: self.config.theme.background,
        }
    }

    fn shape(&self, shape: &Shape, out: &mut Drawn) {
        match shape {
            Shape::Rect {
                at,
                size,
                radius,
                paint,
            } => {
                let (x, y) = self.on_screen(*at);
                let rect = self.painted((x, y, size.0, size.1), *paint);
                out.rects.push(Rect {
                    radius: *radius,
                    ..rect
                });
            }
            Shape::Line {
                from,
                to,
                width,
                paint,
            } => {
                out.rects.push(self.line(*from, *to, *width, *paint));
            }
            Shape::Polygon { points, paint } => self.polygon(points, *paint, out),
            Shape::Text { at, text, paint } => out.labels.push(self.text_shape(*at, text, *paint)),
        }
    }

    pub(crate) fn canvas_layer(&self) -> Drawn {
        let mut out = Drawn::default();
        for shape in &self.status.canvas {
            self.shape(shape, &mut out);
        }
        out
    }

    /// После смены размера окна фигуры у краёв ставятся заново.
    pub(crate) fn redraw_canvas(&mut self) {
        let facts = self.interface_facts();
        self.render_canvas(&facts);
    }

    /// Спрашивает у `draw.lua` фигуры заново.
    pub(crate) fn render_canvas(&mut self, facts: &[(String, String)]) {
        let size = self.window.inner_size();
        let metrics = self.fonts.metrics();
        let mut facts = facts.to_vec();
        let numbers = [
            ("width", size.width),
            ("height", size.height),
            ("cell_width", metrics.cell_width),
            ("cell_height", metrics.cell_height),
        ];
        facts.extend(numbers.map(|(name, value)| (name.to_owned(), value.to_string())));
        let snapshot = self.plugin_snapshot();
        let shapes = self.plugins.as_ref().map(|p| p.draw(&facts, snapshot));
        self.status.canvas = shapes.unwrap_or_default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_lines_and_triangles() {
        assert_eq!((place(-10.0, 100), place(5.0, 100)), (90.0, 5.0));
        let (x, y, w, h, stroke) = segment((10.0, 20.0), (30.0, 10.0), 2.0);
        assert_eq!((x, y, w, h, stroke.rising), (9.0, 9.0, 22.0, 12.0, true));
        let triangle = spans(&[(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)]);
        assert_eq!((triangle.len(), triangle[0]), (10, (0, 0, 10)));
    }
}
