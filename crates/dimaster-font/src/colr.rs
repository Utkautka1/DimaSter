//! Отрисовка векторных цветных глифов (таблицы COLR v0 / v1 и CPAL).

use rustybuzz::ttf_parser::colr::{
    ClipBox, ColorStop, CompositeMode, GradientExtend, GradientStopsIter, Paint, Painter,
};
use rustybuzz::ttf_parser::{Face, GlyphId, OutlineBuilder, RgbaColor, Transform};

use crate::raster::Area;
use crate::{Bitmap, Metrics};

const CURVE_STEPS: u16 = 8;
type LineSeg = ((f32, f32), (f32, f32));

/// `None`, если глиф не описан в таблице COLR.
pub(crate) fn render(
    face: &Face<'_>,
    glyph: GlyphId,
    cells: u32,
    metrics: Metrics,
) -> Option<Bitmap> {
    if !face.is_color_glyph(glyph) {
        return None;
    }
    let (w, h) = (
        (cells * metrics.cell_width) as usize,
        metrics.cell_height as usize,
    );
    let (scale, ox, oy) = fit_transform(face, glyph, w, h)?;
    let root = Transform::new(scale, 0.0, 0.0, -scale, ox, oy);
    let mut painter = ColrPainter::new(face, w, h, root);
    face.paint_color_glyph(glyph, 0, RgbaColor::new(255, 255, 255, 255), &mut painter)?;
    let data = painter.layers.into_iter().next().unwrap_or_default();
    Some(Bitmap {
        left: 0,
        top: metrics.baseline,
        width: w,
        height: h,
        data,
        color: true,
    })
}

fn fit_transform(
    face: &Face<'_>,
    glyph: GlyphId,
    box_w: usize,
    box_h: usize,
) -> Option<(f32, f32, f32)> {
    let upem = f32::from(face.units_per_em().max(1));
    let (xmin, ymin, xmax, ymax) = match face.glyph_bounding_box(glyph) {
        Some(b) => (
            f32::from(b.x_min),
            f32::from(b.y_min),
            f32::from(b.x_max),
            f32::from(b.y_max),
        ),
        None => (
            0.0,
            f32::from(face.descender()),
            upem,
            f32::from(face.ascender()),
        ),
    };
    let (gw, gh) = ((xmax - xmin).max(1.0), (ymax - ymin).max(1.0));
    let scale = (box_w as f32 / gw).min(box_h as f32 / gh) * 0.92;
    let ox = (box_w as f32 - gw * scale) * 0.5 - xmin * scale;
    let oy = (box_h as f32 + gh * scale) * 0.5 + ymin * scale;
    Some((scale, ox, oy))
}

struct ColrPainter<'a> {
    face: &'a Face<'a>,
    width: usize,
    height: usize,
    transforms: Vec<Transform>,
    outline: Vec<LineSeg>,
    layers: Vec<Vec<u8>>,
    layer_modes: Vec<CompositeMode>,
    clips: Vec<Vec<u8>>,
}

impl<'a> ColrPainter<'a> {
    fn new(face: &'a Face<'a>, width: usize, height: usize, root: Transform) -> Self {
        Self {
            face,
            width,
            height,
            transforms: vec![root],
            outline: Vec::new(),
            layers: vec![vec![0; width * height * 4]],
            layer_modes: Vec::new(),
            clips: Vec::new(),
        }
    }

    fn current_transform(&self) -> Transform {
        self.transforms.last().copied().unwrap_or_default()
    }

    fn rasterize_outline(&self) -> Vec<u8> {
        let mut area = Area::new(self.width, self.height);
        for &(from, to) in &self.outline {
            area.line(from, to);
        }
        area.coverage()
    }
}

impl<'a> Painter<'a> for ColrPainter<'a> {
    fn outline_glyph(&mut self, glyph_id: GlyphId) {
        self.outline.clear();
        let mut builder = PathBuilder {
            transform: self.current_transform(),
            lines: &mut self.outline,
            start: (0.0, 0.0),
            last: (0.0, 0.0),
        };
        self.face.outline_glyph(glyph_id, &mut builder);
    }

    fn paint(&mut self, paint: Paint<'a>) {
        let mask = self.rasterize_outline();
        let inv = invert_transform(self.current_transform());
        let Some(dst) = self.layers.last_mut() else {
            return;
        };
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let mut cov = mask[idx];
                for clip in &self.clips {
                    cov = ((u16::from(cov) * u16::from(clip[idx])) / 255) as u8;
                }
                if cov == 0 {
                    continue;
                }
                let src = sample_paint(&paint, x as f32 + 0.5, y as f32 + 0.5, inv);
                blend_over(&mut dst[idx * 4..idx * 4 + 4], src, cov);
            }
        }
    }

    fn push_clip(&mut self) {
        let mask = self.rasterize_outline();
        self.clips.push(mask);
    }

    fn push_clip_box(&mut self, clipbox: ClipBox) {
        let t = self.current_transform();
        let corners = [
            transform_pt(t, clipbox.x_min, clipbox.y_min),
            transform_pt(t, clipbox.x_max, clipbox.y_max),
        ];
        let x0 = corners[0].0.min(corners[1].0).floor().max(0.0) as usize;
        let x1 = (corners[0].0.max(corners[1].0).ceil() as usize).min(self.width);
        let y0 = corners[0].1.min(corners[1].1).floor().max(0.0) as usize;
        let y1 = (corners[0].1.max(corners[1].1).ceil() as usize).min(self.height);
        let mut mask = vec![0u8; self.width * self.height];
        for y in y0..y1 {
            mask[y * self.width + x0..y * self.width + x1].fill(255);
        }
        self.clips.push(mask);
    }

    fn pop_clip(&mut self) {
        self.clips.pop();
    }

    fn push_layer(&mut self, mode: CompositeMode) {
        self.layers.push(vec![0; self.width * self.height * 4]);
        self.layer_modes.push(mode);
    }

    fn pop_layer(&mut self) {
        let Some(top) = self.layers.pop() else { return };
        let mode = self.layer_modes.pop().unwrap_or(CompositeMode::SourceOver);
        if let Some(base) = self.layers.last_mut() {
            composite_layer(base, &top, mode);
        }
    }

    fn push_transform(&mut self, transform: Transform) {
        let combined = Transform::combine(self.current_transform(), transform);
        self.transforms.push(combined);
    }

    fn pop_transform(&mut self) {
        if self.transforms.len() > 1 {
            self.transforms.pop();
        }
    }
}

struct PathBuilder<'a> {
    transform: Transform,
    lines: &'a mut Vec<LineSeg>,
    start: (f32, f32),
    last: (f32, f32),
}

impl PathBuilder<'_> {
    fn pt(&self, x: f32, y: f32) -> (f32, f32) {
        transform_pt(self.transform, x, y)
    }

    fn push(&mut self, to: (f32, f32)) {
        self.lines.push((self.last, to));
        self.last = to;
    }

    fn curve(&mut self, at: impl Fn(f32) -> (f32, f32)) {
        for step in 1..=CURVE_STEPS {
            self.push(at(f32::from(step) / f32::from(CURVE_STEPS)));
        }
    }
}

impl OutlineBuilder for PathBuilder<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.start = self.pt(x, y);
        self.last = self.start;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let to = self.pt(x, y);
        self.push(to);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (a, b, c) = (self.last, self.pt(x1, y1), self.pt(x, y));
        self.curve(|t| lerp2(lerp2(a, b, t), lerp2(b, c, t), t));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, b, c, d) = (self.last, self.pt(x1, y1), self.pt(x2, y2), self.pt(x, y));
        self.curve(|t| {
            let (ab, bc, cd) = (lerp2(a, b, t), lerp2(b, c, t), lerp2(c, d, t));
            lerp2(lerp2(ab, bc, t), lerp2(bc, cd, t), t)
        });
    }

    fn close(&mut self) {
        if self.last != self.start {
            self.push(self.start);
        }
    }
}

fn transform_pt(t: Transform, x: f32, y: f32) -> (f32, f32) {
    (t.a * x + t.c * y + t.e, t.b * x + t.d * y + t.f)
}

fn invert_transform(t: Transform) -> Transform {
    let det = t.a * t.d - t.b * t.c;
    if det.abs() < 1e-8 {
        return Transform::default();
    }
    let inv = 1.0 / det;
    Transform::new(
        t.d * inv,
        -t.b * inv,
        -t.c * inv,
        t.a * inv,
        (t.c * t.f - t.d * t.e) * inv,
        (t.b * t.e - t.a * t.f) * inv,
    )
}

fn lerp2(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

fn sample_paint(paint: &Paint<'_>, px: f32, py: f32, inv: Transform) -> [u8; 4] {
    let (x, y) = transform_pt(inv, px, py);
    match paint {
        Paint::Solid(c) => rgba(*c),
        Paint::LinearGradient(g) => sample_stops(g.stops(0, &[]), g.extend, linear_t(x, y, g)),
        Paint::RadialGradient(g) => sample_stops(g.stops(0, &[]), g.extend, radial_t(x, y, g)),
        Paint::SweepGradient(g) => sample_stops(g.stops(0, &[]), g.extend, sweep_t(x, y, g)),
    }
}

fn linear_t(x: f32, y: f32, g: &rustybuzz::ttf_parser::colr::LinearGradient<'_>) -> f32 {
    let (dx, dy) = (g.x1 - g.x0, g.y1 - g.y0);
    let len2 = dx * dx + dy * dy;
    if len2 > 1e-6 {
        ((x - g.x0) * dx + (y - g.y0) * dy) / len2
    } else {
        0.0
    }
}

fn radial_t(x: f32, y: f32, g: &rustybuzz::ttf_parser::colr::RadialGradient<'_>) -> f32 {
    let d = ((x - g.x0).hypot(y - g.y0) - g.r0)
        / (g.r1 - g.r0 + (g.x1 - g.x0).hypot(g.y1 - g.y0) * 0.001).max(1e-3);
    d.max(0.0)
}

fn sweep_t(x: f32, y: f32, g: &rustybuzz::ttf_parser::colr::SweepGradient<'_>) -> f32 {
    let angle = (y - g.center_y)
        .atan2(x - g.center_x)
        .to_degrees()
        .rem_euclid(360.0);
    (angle - g.start_angle) / (g.end_angle - g.start_angle).max(1e-3)
}

fn sample_stops(mut stops: GradientStopsIter<'_, '_>, extend: GradientExtend, t: f32) -> [u8; 4] {
    let t = match extend {
        GradientExtend::Pad => t.clamp(0.0, 1.0),
        GradientExtend::Repeat => t.rem_euclid(1.0),
        GradientExtend::Reflect => {
            let m = t.rem_euclid(2.0);
            if m > 1.0 { 2.0 - m } else { m }
        }
    };
    let mut prev: Option<ColorStop> = None;
    for stop in &mut stops {
        if t <= stop.stop_offset {
            return match prev {
                Some(p) => lerp_color(p, stop, t),
                None => rgba(stop.color),
            };
        }
        prev = Some(stop);
    }
    prev.map_or([0, 0, 0, 0], |s| rgba(s.color))
}

fn lerp_color(a: ColorStop, b: ColorStop, t: f32) -> [u8; 4] {
    let span = (b.stop_offset - a.stop_offset).max(1e-6);
    let k = ((t - a.stop_offset) / span).clamp(0.0, 1.0);
    let mix = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * k).round() as u8;
    [
        mix(a.color.red, b.color.red),
        mix(a.color.green, b.color.green),
        mix(a.color.blue, b.color.blue),
        mix(a.color.alpha, b.color.alpha),
    ]
}

fn rgba(c: RgbaColor) -> [u8; 4] {
    [c.red, c.green, c.blue, c.alpha]
}

fn blend_over(dst: &mut [u8], src: [u8; 4], cov: u8) {
    let a = (u16::from(src[3]) * u16::from(cov)) / 255;
    let inv = 255 - a;
    for i in 0..3 {
        dst[i] = ((u16::from(src[i]) * a + u16::from(dst[i]) * inv) / 255) as u8;
    }
    dst[3] = dst[3].saturating_add(((a * u16::from(255 - dst[3])) / 255) as u8);
}

fn composite_layer(base: &mut [u8], top: &[u8], mode: CompositeMode) {
    let (dst_chunks, _) = base.as_chunks_mut::<4>();
    let (src_chunks, _) = top.as_chunks::<4>();
    for (dst, src) in dst_chunks.iter_mut().zip(src_chunks.iter()) {
        let s = match mode {
            CompositeMode::Multiply => [
                ((u16::from(dst[0]) * u16::from(src[0])) / 255) as u8,
                ((u16::from(dst[1]) * u16::from(src[1])) / 255) as u8,
                ((u16::from(dst[2]) * u16::from(src[2])) / 255) as u8,
                src[3],
            ],
            _ => [src[0], src[1], src[2], src[3]],
        };
        blend_over(dst, s, 255);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradient_stops_and_blending_work() {
        let mut dst = [0u8, 0, 0, 255];
        blend_over(&mut dst, [200, 100, 50, 255], 128);
        assert!(dst[0] > 90 && dst[1] > 45);
        let c = lerp_color(
            ColorStop {
                stop_offset: 0.0,
                color: RgbaColor::new(0, 0, 0, 255),
            },
            ColorStop {
                stop_offset: 1.0,
                color: RgbaColor::new(200, 100, 50, 255),
            },
            0.5,
        );
        assert_eq!(c, [100, 50, 25, 255]);
    }
}
