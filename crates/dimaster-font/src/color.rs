//! Цветные эмодзи: растр из шрифта, уменьшенный под ячейки терминала.

use dimaster_image::{Image, decode_png};
use rustybuzz::ttf_parser::{self, RasterImageFormat};

use crate::{Bitmap, Metrics};

/// `None`, если у глифа нет PNG-растра (обычный контурный шрифт).
pub(crate) fn render(
    face: &ttf_parser::Face<'_>,
    glyph: ttf_parser::GlyphId,
    cells: u32,
    metrics: Metrics,
) -> Option<Bitmap> {
    let raster = face.glyph_raster_image(glyph, u16::MAX)?;
    if raster.format != RasterImageFormat::PNG {
        return None;
    }
    let image = decode_png(raster.data)?;
    Some(fit(&image, cells * metrics.cell_width, metrics))
}

// вписывает картинку в прямоугольник ячеек с сохранением пропорций и центрирует
fn fit(image: &Image, box_width: u32, metrics: Metrics) -> Bitmap {
    let (box_width, box_height) = (box_width as usize, metrics.cell_height as usize);
    let scale =
        (box_width as f32 / image.width as f32).min(box_height as f32 / image.height as f32);
    let width = ((image.width as f32 * scale) as usize).max(1);
    let height = ((image.height as f32 * scale) as usize).max(1);
    let left = ((box_width - width.min(box_width)) / 2) as i32;
    let top = metrics.baseline - ((box_height - height.min(box_height)) / 2) as i32;
    let data = downscale(image, width, height);
    Bitmap {
        left,
        top,
        width,
        height,
        data,
        color: true,
    }
}

// усреднение по площади: при уменьшении в 5 раз ближайший сосед даёт «шум»
fn downscale(image: &Image, width: usize, height: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        let rows = span(y, height, image.height);
        for x in 0..width {
            out.extend(average(image, span(x, width, image.width), rows.clone()));
        }
    }
    out
}

fn span(index: usize, count: usize, source: usize) -> std::ops::Range<usize> {
    let start = index * source / count;
    start..((index + 1) * source / count).max(start + 1).min(source)
}

// цвет взвешивается по альфе, иначе прозрачные пиксели темнят края
fn average(
    image: &Image,
    columns: std::ops::Range<usize>,
    rows: std::ops::Range<usize>,
) -> [u8; 4] {
    let mut sum = [0u32; 4];
    let count = (columns.len() * rows.len()) as u32;
    for y in rows {
        for x in columns.clone() {
            let p = &image.pixels[(y * image.width + x) * 4..][..4];
            let alpha = u32::from(p[3]);
            (0..3).for_each(|i| sum[i] += u32::from(p[i]) * alpha);
            sum[3] += alpha;
        }
    }
    let channel = |i: usize| sum[i].checked_div(sum[3]).unwrap_or(0) as u8;
    [channel(0), channel(1), channel(2), (sum[3] / count) as u8]
}
