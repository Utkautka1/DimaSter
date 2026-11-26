//! Размещение картинок в сетке и их удаление.

use std::sync::Arc;

use super::{MAX_IMAGE_ROWS, Picture, Placement};
use crate::terminal::State;

// размер ячейки, когда окна нет (headless и тесты)
const FALLBACK_CELL: (usize, usize) = (10, 20);
// защита от потока картинок в одну и ту же строку
const PER_ROW: usize = 64;

impl State {
    pub(crate) fn cell_size(&self) -> (usize, usize) {
        match self.cell_pixels {
            (0, _) | (_, 0) => FALLBACK_CELL,
            size => size,
        }
    }

    /// Размер в ячейках; недостающая сторона вычисляется с сохранением пропорций.
    pub(crate) fn image_cells(
        &self,
        picture: &Picture,
        cols: Option<usize>,
        rows: Option<usize>,
    ) -> (usize, usize) {
        let (cell_width, cell_height) = self.cell_size();
        let width = picture.animation.width.max(1);
        let height = picture.animation.height.max(1);
        let (cols, rows) = match (cols.filter(|&c| c > 0), rows.filter(|&r| r > 0)) {
            (Some(cols), Some(rows)) => (cols, rows),
            (Some(cols), None) => (
                cols,
                (cols * cell_width * height).div_ceil(width * cell_height),
            ),
            (None, Some(rows)) => (
                (rows * cell_height * width).div_ceil(height * cell_width),
                rows,
            ),
            (None, None) => (width.div_ceil(cell_width), height.div_ceil(cell_height)),
        };
        self.fit_width(cols.max(1), rows.max(1))
    }

    // слишком широкая картинка уменьшается до правого края экрана
    fn fit_width(&self, cols: usize, rows: usize) -> (usize, usize) {
        let available = (self.screen.cols() - self.cursor.col).max(1);
        let rows = if cols > available {
            (rows * available).div_ceil(cols)
        } else {
            rows
        };
        (cols.min(available), rows.clamp(1, MAX_IMAGE_ROWS))
    }

    /// Показывает картинку в позиции курсора, не двигая его.
    pub(crate) fn place(&mut self, picture: Arc<Picture>, cells: (usize, usize), id: (u32, u32)) {
        if id.0 != 0 && id.1 != 0 {
            self.remove_images(|p| p.id == id);
        }
        let col = self.cursor.col;
        let images = &mut self.screen.rows[self.cursor.row].images;
        images.retain(|p| p.col != col);
        if images.len() == PER_ROW {
            images.remove(0);
        }
        let (cols, rows) = cells;
        images.push(Placement {
            col,
            cols,
            rows,
            picture,
            id,
        });
    }

    /// Как в Kitty: курсор оказывается правее нижней строки картинки.
    pub(crate) fn place_and_advance(
        &mut self,
        picture: Arc<Picture>,
        cells: (usize, usize),
        id: (u32, u32),
    ) {
        self.place(picture, cells, id);
        let col = self.cursor.col + cells.0;
        // line_feed прокручивает экран, и якорь уезжает вместе с текстом
        (1..cells.1).for_each(|_| self.line_feed());
        self.set_col(col);
    }

    pub(crate) fn remove_images(&mut self, doomed: impl Fn(&Placement) -> bool) {
        let history = self.screen.history.iter_mut().rev().take(MAX_IMAGE_ROWS);
        for row in self.screen.rows.iter_mut().chain(history) {
            row.images.retain(|p| !doomed(p));
        }
    }

    /// Очистка экрана убирает и картинки, свисающие в него из истории.
    pub(crate) fn remove_overhanging_images(&mut self) {
        for (distance, row) in self.screen.history.iter_mut().rev().enumerate() {
            if distance >= MAX_IMAGE_ROWS {
                break;
            }
            row.images.retain(|p| p.rows <= distance + 1);
        }
    }
}
