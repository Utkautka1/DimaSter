//! Встроенные картинки iTerm2 (OSC 1337 `File=`).

use dimaster_image::{Animation, decode_base64};

use super::Then;
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

// значение из заголовка вида `ключ=значение;…`
fn field<'a>(header: &'a str, key: &str) -> Option<&'a str> {
    header
        .split(';')
        .find_map(|p| p.strip_prefix(key)?.strip_prefix('='))
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
        let header = String::from_utf8_lossy(&rest[..split]).into_owned();
        if field(&header, "inline") != Some("1") {
            return;
        }
        if let Some(file) = decode_base64(&rest[split + 1..]) {
            self.decode_or_defer(file, Then::Iterm(header));
        }
    }

    pub(super) fn show_iterm(&mut self, header: &str, animation: Option<Animation>) {
        let Some(animation) = animation else { return };
        let (cell, rows) = (self.cell_size(), self.screen.rows.len());
        let width = field(header, "width").and_then(|v| cells(v, cell.0, self.screen.cols()));
        let height = field(header, "height").and_then(|v| cells(v, cell.1, rows));
        let picture = self.graphics.picture(animation);
        let size = self.image_cells(&picture, width, height, None);
        self.place_and_advance(picture, size, (0, 0), None);
    }
}
