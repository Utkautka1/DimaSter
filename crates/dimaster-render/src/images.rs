//! Картинки в кадре: где рисовать, обрезка, плейсхолдеры `U=1` и кадр анимации.

use std::sync::Arc;
use std::time::Duration;

use dimaster_grid::{
    MAX_IMAGE_ROWS, Picture, PlaceholderCell, Placement, Row, Terminal, placeholder_info,
};

use crate::frame::{Builder, Quad};

/// Бит цвета, по которому шейдер берёт всю текстуру с фильтрацией.
const IMAGE_FLAG: u32 = 1 << 25;
// младший байт — непрозрачность
const OPAQUE: u32 = IMAGE_FLAG | 255;

/// Прямоугольник с картинкой; рисуется отдельным вызовом со своей текстурой.
pub(crate) struct ImageQuad {
    pub(crate) picture: Arc<Picture>,
    pub(crate) frame: usize,
    pub(crate) quad: Quad,
}

/// Все картинки кадра и время до ближайшей смены кадра анимации.
#[derive(Default)]
pub(crate) struct Images {
    pub(crate) quads: Vec<ImageQuad>,
    pub(crate) next_frame: Option<Duration>,
}

impl Images {
    pub(crate) fn clear(&mut self) {
        self.quads.clear();
        self.next_frame = None;
    }
}

// вписывает картинку в прямоугольник ячеек с сохранением пропорций
fn fit(natural: [f32; 2], width: f32, height: f32) -> [f32; 2] {
    let scale = (width / natural[0].max(1.0)).min(height / natural[1].max(1.0));
    [natural[0] * scale, natural[1] * scale]
}

fn crop_uv(picture: &Picture, crop: Option<[u32; 4]>) -> ([f32; 2], [f32; 2], [f32; 2]) {
    let (pw, ph) = picture.dimensions();
    let (pw, ph) = (pw.max(1) as f32, ph.max(1) as f32);
    match crop {
        Some([x, y, w, h]) => (
            [x as f32 / pw, y as f32 / ph],
            [w as f32 / pw, h as f32 / ph],
            [w as f32, h as f32],
        ),
        None => ([0.0, 0.0], [1.0, 1.0], [pw, ph]),
    }
}

impl Builder<'_> {
    /// Фоновая картинка заполняет окно целиком с сохранением пропорций.
    pub(crate) fn wallpaper(&mut self, window: [f32; 2]) -> Option<ImageQuad> {
        let (picture, strength) = self.look.wallpaper.clone()?;
        let (pw, ph) = picture.dimensions();
        let natural = [pw.max(1) as f32, ph.max(1) as f32];
        let scale = (window[0] / natural[0]).max(window[1] / natural[1]);
        let size = [natural[0] * scale, natural[1] * scale];
        let position = [(window[0] - size[0]) / 2.0, (window[1] - size[1]) / 2.0];
        let color = IMAGE_FLAG | (strength.clamp(0.0, 1.0) * 255.0) as u32;
        let quad = Quad {
            position,
            size,
            uv: [0.0, 0.0],
            color,
            uv_size: [1.0, 1.0],
        };
        let frame = self.animation_frame(&picture);
        Some(ImageQuad {
            picture,
            frame,
            quad,
        })
    }

    /// Картинки, начавшиеся выше экрана, тоже могут в него свисать, плюс плейсхолдеры.
    pub(crate) fn images(&mut self, term: &Terminal, first: u64) {
        let rows = term.screen().rows().len() as u64;
        let start = first.saturating_sub(MAX_IMAGE_ROWS as u64);
        for line in start..first + rows {
            let Some(row) = term.line(line) else { continue };
            let offset = line as i64 - first as i64;
            for placement in row.images.iter().filter(|p| offset + p.rows as i64 > 0) {
                self.image(placement, offset);
            }
            if offset >= 0 {
                self.placeholders(term, &row, offset as usize);
            }
        }
    }

    fn image(&mut self, placement: &Placement, row: i64) {
        let m = self.fonts.metrics();
        let (cw, ch) = (m.cell_width as f32, m.cell_height as f32);
        let pos = [
            self.look.padding.0 as f32 + placement.col as f32 * cw,
            (self.look.padding.1 + self.look.top) as f32 + row as f32 * ch,
        ];
        let (uv, uv_size, natural) = crop_uv(&placement.picture, placement.crop);
        let size = fit(
            natural,
            placement.cols as f32 * cw,
            placement.rows as f32 * ch,
        );
        let picture = placement.picture.clone();
        let frame = self.animation_frame(&picture);
        self.pictures.quads.push(ImageQuad {
            picture,
            frame,
            quad: Quad {
                position: pos,
                size,
                uv,
                color: OPAQUE,
                uv_size,
            },
        });
    }

    // собирает соседние плейсхолдеры `U+10EEEE` одной строки в единый прямоугольник
    fn placeholders(&mut self, term: &Terminal, row: &Row, screen_row: usize) {
        let mut col = 0;
        while col < row.cells.len() {
            let Some(first) = placeholder_info(&row.cells[col]) else {
                col += 1;
                continue;
            };
            let span = placeholder_span(&row.cells[col..], first);
            self.push_placeholder(term, first, (screen_row, col), span);
            col += span;
        }
    }

    fn push_placeholder(
        &mut self,
        term: &Terminal,
        info: PlaceholderCell,
        at: (usize, usize),
        span: usize,
    ) {
        let Some(vp) = term.virtual_placement(info.image_id, info.placement_id) else {
            return;
        };
        if info.row >= vp.rows || info.col >= vp.cols {
            return;
        }
        let span = span.min(vp.cols - info.col);
        let quad = self.placeholder_quad(info, (vp.cols, vp.rows), at, span);
        let frame = self.animation_frame(&vp.picture);
        self.pictures.quads.push(ImageQuad {
            picture: vp.picture,
            frame,
            quad,
        });
    }

    fn placeholder_quad(
        &self,
        info: PlaceholderCell,
        grid: (usize, usize),
        (row, col): (usize, usize),
        span: usize,
    ) -> Quad {
        let m = self.fonts.metrics();
        let (cw, ch) = (m.cell_width as f32, m.cell_height as f32);
        Quad {
            position: [
                self.look.padding.0 as f32 + col as f32 * cw,
                (self.look.padding.1 + self.look.top) as f32 + row as f32 * ch,
            ],
            size: [span as f32 * cw, ch],
            uv: [
                info.col as f32 / grid.0 as f32,
                info.row as f32 / grid.1 as f32,
            ],
            color: OPAQUE,
            uv_size: [span as f32 / grid.0 as f32, 1.0 / grid.1 as f32],
        }
    }

    // выбирает кадр и запоминает, когда понадобится следующий
    fn animation_frame(&mut self, picture: &Picture) -> usize {
        let (frame, wait) = picture.current_frame();
        if let Some(wait) = wait {
            let soonest = self.pictures.next_frame.map_or(wait, |w| w.min(wait));
            self.pictures.next_frame = Some(soonest);
        }
        frame
    }
}

fn placeholder_span(cells: &[dimaster_grid::Cell], first: PlaceholderCell) -> usize {
    cells
        .iter()
        .enumerate()
        .take_while(|(offset, cell)| {
            placeholder_info(cell).is_some_and(|p| {
                p.image_id == first.image_id
                    && p.placement_id == first.placement_id
                    && p.row == first.row
                    && p.col == first.col + *offset
            })
        })
        .count()
        .max(1)
}
