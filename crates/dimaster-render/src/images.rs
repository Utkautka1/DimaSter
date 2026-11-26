//! Картинки в кадре: где рисовать и какой кадр анимации сейчас.

use std::sync::Arc;
use std::time::Duration;

use dimaster_grid::{MAX_IMAGE_ROWS, Picture, Placement, Terminal};

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
fn fit(picture: &Picture, width: f32, height: f32) -> [f32; 2] {
    let natural = [
        picture.animation.width as f32,
        picture.animation.height as f32,
    ];
    let scale = (width / natural[0]).min(height / natural[1]);
    [natural[0] * scale, natural[1] * scale]
}

impl Builder<'_> {
    /// Фоновая картинка заполняет окно целиком с сохранением пропорций.
    pub(crate) fn wallpaper(&mut self, window: [f32; 2]) -> Option<ImageQuad> {
        let (picture, strength) = self.look.wallpaper.clone()?;
        let natural = [
            picture.animation.width as f32,
            picture.animation.height as f32,
        ];
        let scale = (window[0] / natural[0]).max(window[1] / natural[1]);
        let size = [natural[0] * scale, natural[1] * scale];
        let position = [(window[0] - size[0]) / 2.0, (window[1] - size[1]) / 2.0];
        let color = IMAGE_FLAG | (strength.clamp(0.0, 1.0) * 255.0) as u32;
        let quad = Quad {
            position,
            size,
            uv: [0.0, 0.0],
            color,
        };
        let frame = self.animation_frame(&picture);
        Some(ImageQuad {
            picture,
            frame,
            quad,
        })
    }

    /// Картинки, начавшиеся выше экрана, тоже могут в него свисать.
    pub(crate) fn images(&mut self, term: &Terminal, first: u64) {
        let rows = term.screen().rows().len() as u64;
        let start = first.saturating_sub(MAX_IMAGE_ROWS as u64);
        for line in start..first + rows {
            let Some(row) = term.line(line) else { continue };
            let offset = line as i64 - first as i64;
            for placement in row.images.iter().filter(|p| offset + p.rows as i64 > 0) {
                self.image(placement, offset);
            }
        }
    }

    fn image(&mut self, placement: &Placement, row: i64) {
        let metrics = self.fonts.metrics();
        let (cell_width, cell_height) = (metrics.cell_width as f32, metrics.cell_height as f32);
        let position = [
            self.look.padding.0 as f32 + placement.col as f32 * cell_width,
            self.look.padding.1 as f32 + row as f32 * cell_height,
        ];
        let width = placement.cols as f32 * cell_width;
        let size = fit(
            &placement.picture,
            width,
            placement.rows as f32 * cell_height,
        );
        let picture = placement.picture.clone();
        let frame = self.animation_frame(&picture);
        let quad = Quad {
            position,
            size,
            uv: [0.0, 0.0],
            color: OPAQUE,
        };
        self.pictures.quads.push(ImageQuad {
            picture,
            frame,
            quad,
        });
    }

    // выбирает кадр и запоминает, когда понадобится следующий
    fn animation_frame(&mut self, picture: &Picture) -> usize {
        let (frame, wait) = picture.animation.frame_at(picture.started.elapsed());
        if let Some(wait) = wait {
            let soonest = self.pictures.next_frame.map_or(wait, |w| w.min(wait));
            self.pictures.next_frame = Some(soonest);
        }
        frame
    }
}
