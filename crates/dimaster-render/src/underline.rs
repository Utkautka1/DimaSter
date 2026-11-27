//! Подчёркивания всех видов и зачёркивание.

use dimaster_font::Metrics;
use dimaster_grid::{Cell, Flags, Underline};

use crate::frame::{Builder, Quad};
use crate::palette::Rgb;

impl Builder<'_> {
    pub(crate) fn lines(&mut self, origin: [f32; 2], cell: &Cell, text: Rgb) {
        let Metrics {
            baseline,
            line_thickness,
            cell_height,
            ..
        } = self.fonts.metrics();
        let (width, thick) = (self.cell_width(cell), line_thickness as f32);
        let style = cell.style;
        // волна и двойная линия не должны выходить за нижний край ячейки
        let low = cell_height
            .saturating_sub(line_thickness * dimaster_limits::UNDERLINE_WAVE_ROOM.u32())
            as f32;
        let y = origin[1] + (baseline as f32 + thick).min(low);
        let color = self.underline_color(style, text);
        self.underline(style.underline, [origin[0], y], width, color);
        if style.flags.contains(Flags::STRIKE) {
            let at = [
                origin[0],
                origin[1] + (baseline * dimaster_limits::STRIKE_POSITION.get() as i32 / 100) as f32,
            ];
            self.quads.push(Quad::solid(at, [width, thick], text));
        }
    }

    fn dashes(&mut self, at: [f32; 2], width: f32, color: Rgb) {
        let part = |percent: u64| width * percent as f32 / 100.0;
        let step = part(dimaster_limits::UNDERLINE_DASH_PERIOD.get());
        let length = part(dimaster_limits::UNDERLINE_DASH_LENGTH.get());
        self.pieces(at, width, step, length, color);
    }

    fn double(&mut self, [x, y]: [f32; 2], width: f32, color: Rgb) {
        let thick = self.fonts.metrics().line_thickness as f32;
        let gap = thick * dimaster_limits::UNDERLINE_DOUBLE_GAP.get() as f32;
        for at in [[x, y], [x, y + gap]] {
            self.quads.push(Quad::solid(at, [width, thick], color));
        }
    }

    fn underline(&mut self, kind: Underline, [x, y]: [f32; 2], width: f32, color: Rgb) {
        let thick = self.fonts.metrics().line_thickness as f32;
        let full = [width, thick];
        match kind {
            Underline::None => {}
            Underline::Single => self.quads.push(Quad::solid([x, y], full, color)),
            Underline::Double => self.double([x, y], width, color),
            Underline::Curly => self.wave([x, y], width, color),
            Underline::Dotted => self.pieces(
                [x, y],
                width,
                thick * dimaster_limits::UNDERLINE_DOT_STEP.get() as f32,
                thick,
                color,
            ),
            Underline::Dashed => self.dashes([x, y], width, color),
        }
    }

    // штрихи длиной `length` с шагом `step`
    fn pieces(&mut self, [x, y]: [f32; 2], width: f32, step: f32, length: f32, color: Rgb) {
        let thick = self.fonts.metrics().line_thickness as f32;
        let mut offset = 0.0;
        while offset < width {
            let size = [length.min(width - offset).round(), thick];
            self.quads
                .push(Quad::solid([x + offset.round(), y], size, color));
            offset += step.max(1.0);
        }
    }

    // синусоида из столбиков по пикселю; фаза от края ячейки, клетки стыкуются
    fn wave(&mut self, [x, y]: [f32; 2], width: f32, color: Rgb) {
        let thick = self.fonts.metrics().line_thickness as f32;
        let period = self.fonts.metrics().cell_width as f32;
        for column in 0..width as u32 {
            let at = x + column as f32;
            let phase = (column as f32 / period * std::f32::consts::TAU).sin();
            let rise = ((phase + 1.0) * thick).round();
            self.quads
                .push(Quad::solid([at, y + rise], [1.0, thick], color));
        }
    }
}
