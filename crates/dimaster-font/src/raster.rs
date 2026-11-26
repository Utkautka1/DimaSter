//! Растеризатор контуров с точным покрытием пикселей.
//!
//! Каждый отрезок контура добавляет в буфер знаковую площадь, а префиксная
//! сумма по строке даёт долю закрашенности пикселя. Это даёт сглаживание без
//! суперсэмплинга и без хранения списков пересечений.

use ttf_parser::OutlineBuilder;

/// Растр глифа: покрытие 0..=255 и положение относительно базовой линии.
#[derive(Debug, Clone, Default)]
pub struct Bitmap {
    pub left: i32,
    /// Расстояние от базовой линии до верхнего края (вверх — положительное).
    pub top: i32,
    pub width: usize,
    pub height: usize,
    pub coverage: Vec<u8>,
}

type Point = (f32, f32);

// на столько отрезков делится кривая Безье; хватает для кеглей до ~100 px
const CURVE_STEPS: u16 = 8;

/// Собирает контур в пиксельных координатах как набор отрезков.
#[derive(Debug, Default)]
pub(crate) struct Outline {
    // множитель из единиц шрифта в пиксели
    scale: f32,
    lines: Vec<(Point, Point)>,
    start: Point,
    last: Point,
}

impl Outline {
    pub(crate) fn new(scale: f32) -> Self {
        Self {
            scale,
            ..Self::default()
        }
    }

    fn point(&self, x: f32, y: f32) -> Point {
        (x * self.scale, y * self.scale)
    }

    fn push(&mut self, to: Point) {
        self.lines.push((self.last, to));
        self.last = to;
    }

    fn curve(&mut self, at: impl Fn(f32) -> Point) {
        for step in 1..=CURVE_STEPS {
            let point = at(f32::from(step) / f32::from(CURVE_STEPS));
            self.push(point);
        }
    }

    /// Превращает накопленный контур в растр.
    pub(crate) fn rasterize(&self) -> Bitmap {
        let Some((min, max)) = self.bounds() else {
            return Bitmap::default();
        };
        let (left, top) = (min.0.floor(), max.1.ceil());
        let width = (max.0.ceil() - left) as usize;
        let height = (top - min.1.floor()) as usize;
        let mut area = Area::new(width, height);
        for &(from, to) in &self.lines {
            // переносим в координаты растра: начало сверху слева, ось Y вниз
            area.line((from.0 - left, top - from.1), (to.0 - left, top - to.1));
        }
        let coverage = area.coverage();
        Bitmap {
            left: left as i32,
            top: top as i32,
            width,
            height,
            coverage,
        }
    }

    fn bounds(&self) -> Option<(Point, Point)> {
        let mut points = self.lines.iter().flat_map(|&(a, b)| [a, b]);
        let first = points.next()?;
        Some(points.fold((first, first), |(min, max), p| {
            (
                (min.0.min(p.0), min.1.min(p.1)),
                (max.0.max(p.0), max.1.max(p.1)),
            )
        }))
    }
}

impl OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.start = self.point(x, y);
        self.last = self.start;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let to = self.point(x, y);
        self.push(to);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (a, b, c) = (self.last, self.point(x1, y1), self.point(x, y));
        self.curve(|t| lerp(lerp(a, b, t), lerp(b, c, t), t));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, b) = (self.last, self.point(x1, y1));
        let (c, d) = (self.point(x2, y2), self.point(x, y));
        self.curve(|t| {
            let (ab, bc, cd) = (lerp(a, b, t), lerp(b, c, t), lerp(c, d, t));
            lerp(lerp(ab, bc, t), lerp(bc, cd, t), t)
        });
    }

    fn close(&mut self) {
        if self.last != self.start {
            let start = self.start;
            self.push(start);
        }
    }
}

fn lerp(a: Point, b: Point, t: f32) -> Point {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Буфер приращений покрытия.
struct Area {
    width: usize,
    height: usize,
    // два запасных столбца: вклад правее последнего пикселя не заезжает в строку ниже
    stride: usize,
    cells: Vec<f32>,
}

impl Area {
    fn new(width: usize, height: usize) -> Self {
        let stride = width + 2;
        Self {
            width,
            height,
            stride,
            cells: vec![0.0; stride * height],
        }
    }

    /// Режет отрезок по строкам пикселей.
    fn line(&mut self, from: Point, to: Point) {
        if from.1 == to.1 {
            return;
        }
        let (sign, top, bottom) = if from.1 < to.1 {
            (1.0, from, to)
        } else {
            (-1.0, to, from)
        };
        let slope = (bottom.0 - top.0) / (bottom.1 - top.1);
        let last = (bottom.1.ceil() as usize).min(self.height);
        for row in top.1.max(0.0) as usize..last {
            let y0 = top.1.max(row as f32);
            let y1 = bottom.1.min(row as f32 + 1.0);
            let x0 = top.0 + slope * (y0 - top.1);
            let x1 = top.0 + slope * (y1 - top.1);
            self.span(row, (x0.min(x1), x0.max(x1)), sign * (y1 - y0));
        }
    }

    // Раздаёт высоту отрезка `delta` пикселям строки между x0 и x1
    fn span(&mut self, row: usize, (x0, x1): (f32, f32), delta: f32) {
        let (x0, x1) = (
            x0.clamp(0.0, self.width as f32),
            x1.clamp(0.0, self.width as f32),
        );
        let first = x0.floor() as usize;
        let last = (x1.ceil() as usize).max(first + 1);
        let base = row * self.stride;
        for col in first..last {
            let left = x0.max(col as f32);
            let right = x1.min(col as f32 + 1.0);
            let share = if x1 > x0 {
                (right - left) / (x1 - x0)
            } else {
                1.0
            };
            // часть пикселя правее отрезка закрашена сразу, остальное — со следующего
            let inside = col as f32 + 1.0 - (left + right) / 2.0;
            self.cells[base + col] += delta * share * inside;
            self.cells[base + col + 1] += delta * share * (1.0 - inside);
        }
    }

    fn coverage(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.width * self.height);
        for row in self.cells.chunks(self.stride).take(self.height) {
            let mut sum = 0.0;
            out.extend(row[..self.width].iter().map(|&delta| {
                sum += delta;
                (sum.abs().min(1.0) * 255.0).round() as u8
            }));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn polygon(points: &[Point]) -> Bitmap {
        let mut outline = Outline::new(1.0);
        outline.move_to(points[0].0, points[0].1);
        points[1..].iter().for_each(|p| outline.line_to(p.0, p.1));
        outline.close();
        outline.rasterize()
    }

    #[test]
    fn half_pixel_square_has_partial_coverage() {
        let bitmap = polygon(&[(0.5, 0.0), (2.0, 0.0), (2.0, 1.0), (0.5, 1.0)]);
        assert_eq!((bitmap.width, bitmap.height), (2, 1));
        assert_eq!(bitmap.coverage, [128, 255]);
    }

    #[test]
    fn triangle_area_matches_geometry() {
        let bitmap = polygon(&[(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)]);
        let total: u32 = bitmap.coverage.iter().map(|&c| u32::from(c)).sum();
        assert!((total as f32 / 255.0 - 50.0).abs() < 0.5);
    }
}
