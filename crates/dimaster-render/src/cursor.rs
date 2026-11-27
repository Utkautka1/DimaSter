//! Курсор и его хвост.

use dimaster_font::Metrics;
use dimaster_grid::{CursorShape, Terminal};

use crate::appearance::CursorLook;
use crate::frame::{Builder, Quad};
use crate::palette::Rgb;
use crate::text::Run;

/// Место и размер прямоугольника курсора внутри ячейки `cell`.
fn geometry(
    shape: CursorShape,
    look: CursorLook,
    origin: [f32; 2],
    cell: [f32; 2],
    line: f32,
) -> ([f32; 2], [f32; 2]) {
    let thick = match look.thickness {
        0 => line,
        pixels => pixels as f32,
    };
    let (width, height) = (cell[0] * look.width, cell[1] * look.height);
    let bottom = origin[1] + cell[1];
    match shape {
        CursorShape::Block => ([origin[0], bottom - height], [width, height]),
        CursorShape::Underline => ([origin[0], bottom - thick], [width, thick]),
        // черта укорачивается равномерно сверху и снизу
        CursorShape::Bar => (
            [origin[0], origin[1] + (cell[1] - height) / 2.0],
            [thick, height],
        ),
    }
}

// сжатие к середине по высоте
fn squeeze((at, size): ([f32; 2], [f32; 2]), shrink: f32) -> ([f32; 2], [f32; 2]) {
    let cut = size[1] * shrink.clamp(0.0, 1.0);
    (
        [at[0], at[1] + cut / 2.0],
        [size[0], (size[1] - cut).max(1.0)],
    )
}

// четыре стороны рамки
fn outline([x, y]: [f32; 2], [width, height]: [f32; 2], line: f32) -> [([f32; 2], [f32; 2]); 4] {
    [
        ([x, y], [width, line]),
        ([x, y + height - line], [width, line]),
        ([x, y], [line, height]),
        ([x + width - line, y], [line, height]),
    ]
}

impl Builder<'_> {
    fn shaped(&mut self, at: [f32; 2], size: [f32; 2], color: Rgb, shape: (f32, u32)) {
        let mut quad = Quad::solid(at, size, color);
        // у заливки координаты атласа свободны: в них непрозрачность и радиус
        quad.uv = [-shape.0, shape.1 as f32];
        self.quads.push(quad);
    }

    fn cell_origin(&self, row: f32, col: f32) -> [f32; 2] {
        let (metrics, zero) = (self.fonts.metrics(), self.origin(0, 0));
        [
            zero[0] + col * metrics.cell_width as f32,
            zero[1] + row * metrics.cell_height as f32,
        ]
    }

    pub(crate) fn trail(&mut self, term: &Terminal, trail: &[(f32, f32)]) {
        let Metrics {
            cell_width,
            cell_height,
            line_thickness,
            ..
        } = self.fonts.metrics();
        let look = self.look.cursor_look;
        let color = look.trail_color.or(look.color).unwrap_or(self.look.cursor);
        let cell = [cell_width as f32, cell_height as f32];
        let line = (line_thickness * 2) as f32;
        for &(row, col) in trail {
            let origin = self.cell_origin(row, col);
            let (at, size) = geometry(term.modes().cursor_shape, look, origin, cell, line);
            self.shaped(at, size, color, (look.trail_opacity, look.radius));
        }
    }

    fn cursor_body(
        &mut self,
        shape: CursorShape,
        origin: [f32; 2],
        width: f32,
        (hollow, shrink): (bool, f32),
    ) {
        let metrics = self.fonts.metrics();
        let cell = [width, metrics.cell_height as f32];
        let look = self.look.cursor_look;
        let color = look.color.unwrap_or(self.look.cursor);
        let line = (metrics.line_thickness * 2) as f32;
        let (at, size) = squeeze(geometry(shape, look, origin, cell, line), shrink);
        if !hollow {
            return self.shaped(at, size, color, (look.opacity, look.radius));
        }
        for (at, size) in outline(at, size, metrics.line_thickness.max(1) as f32) {
            self.shaped(at, size, color, (look.opacity, 0));
        }
    }

    pub(crate) fn cursor(
        &mut self,
        term: &Terminal,
        moved: Option<(f32, f32)>,
        (unfocused, shrink): (bool, f32),
    ) {
        let cursor = term.cursor();
        let cells = &term.screen().rows()[cursor.row].cells;
        let cell = &cells[cursor.col];
        let order = self.visual_order(cells);
        let column = order.map_or(cursor.col, |o| o[cursor.col]);
        let resting = (cursor.row as f32, column as f32);
        let (row, col) = moved.unwrap_or(resting);
        let hollow = unfocused && self.look.cursor_look.hollow_unfocused;
        let shape = term.modes().cursor_shape;
        let origin = self.cell_origin(row, col);
        self.cursor_body(shape, origin, self.cell_width(cell), (hollow, shrink));
        // под неподвижным блоком символ перерисовывается контрастным цветом
        if shape == CursorShape::Block && !hollow && (row, col) == resting {
            let color = self
                .look
                .cursor_look
                .text_color
                .unwrap_or(self.look.background);
            let mut run = Run::default();
            self.text_cell(&mut run, cell, color, (cursor.row, column), false);
            self.flush(&mut run, cursor.row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_keeps_the_middle() {
        let squeezed = squeeze(([0.0, 10.0], [2.0, 20.0]), 0.5);
        assert_eq!(squeezed, ([0.0, 15.0], [2.0, 10.0]));
    }

    #[test]
    fn shapes_respect_size_and_thickness() {
        let (origin, cell) = ([10.0, 20.0], [8.0, 16.0]);
        let mut look = CursorLook {
            height: 0.5,
            ..CursorLook::default()
        };
        let block = geometry(CursorShape::Block, look, origin, cell, 2.0);
        assert_eq!(block, ([10.0, 28.0], [8.0, 8.0]));
        let bar = geometry(CursorShape::Bar, look, origin, cell, 2.0);
        assert_eq!(bar, ([10.0, 24.0], [2.0, 8.0]));
        look.thickness = 5;
        let under = geometry(CursorShape::Underline, look, origin, cell, 2.0);
        assert_eq!(under, ([10.0, 31.0], [8.0, 5.0]));
    }
}
