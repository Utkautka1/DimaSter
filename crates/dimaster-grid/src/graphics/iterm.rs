//! Встроенные картинки iTerm2 (OSC 1337 `File=`).

use dimaster_image::{decode, decode_base64};

use crate::terminal::State;

// `N` — ячейки, `Npx` — пиксели, `N%` — доля экрана, `auto` — по картинке
fn cells(value: &str, cell: usize, total: usize) -> Option<usize> {
    if let Some(pixels) = value.strip_suffix("px") {
        return Some(pixels.parse::<usize>().ok()?.div_ceil(cell));
    }
    if let Some(percent) = value.strip_suffix('%') {
        return Some(total * percent.parse::<usize>().ok()?.min(100) / 100);
    }
    value.parse().ok()
}

impl State {
    /// `payload` — всё после `1337;`.
    pub(crate) fn iterm(&mut self, payload: &[u8]) {
        let Some(rest) = payload.strip_prefix(b"File=") else {
            return;
        };
        let Some(split) = rest.iter().position(|&b| b == b':') else {
            return;
        };
        let header = String::from_utf8_lossy(&rest[..split]);
        let value = |key: &str| {
            header
                .split(';')
                .find_map(|p| p.strip_prefix(key)?.strip_prefix('='))
        };
        if value("inline") != Some("1") {
            return;
        }
        let Some(animation) = decode_base64(&rest[split + 1..]).and_then(|d| decode(&d)) else {
            return;
        };
        let (cell, rows) = (self.cell_size(), self.screen.rows.len());
        let width = value("width").and_then(|v| cells(v, cell.0, self.screen.cols()));
        let height = value("height").and_then(|v| cells(v, cell.1, rows));
        let picture = self.graphics.picture(animation);
        let size = self.image_cells(&picture, width, height);
        self.place_and_advance(picture, size, (0, 0));
    }
}
