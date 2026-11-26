//! Картинки на экране.
//!
//! Картинка привязана к строке, на которой начинается: так она сама
//! прокручивается вместе с текстом, уходит в историю, переживает reflow и
//! освобождается, когда строка вытесняется.

mod iterm;
mod kitty;
mod place;
mod store;

use std::sync::Arc;
use std::time::Instant;

use dimaster_image::{Animation, decode, decode_base64, decode_sixel};

pub(crate) use kitty::Pending;
pub(crate) use store::Store;

use crate::terminal::State;

/// Декодированная картинка, общая для всех мест, где она показана.
#[derive(Debug)]
pub struct Picture {
    /// Уникальный номер — по нему рендер кэширует текстуры.
    pub serial: u64,
    pub animation: Animation,
    /// Начало отсчёта анимации.
    pub started: Instant,
}

/// Картинка, показанная начиная с колонки строки-якоря.
#[derive(Debug, Clone)]
pub struct Placement {
    pub col: usize,
    // размер в ячейках
    pub cols: usize,
    pub rows: usize,
    pub picture: Arc<Picture>,
    // номера картинки и размещения из протокола; (0, 0) — безымянное
    pub(crate) id: (u32, u32),
}

/// Максимальная высота картинки в строках.
pub const MAX_IMAGE_ROWS: usize = 500;

impl State {
    /// APC: `G` — протокол Kitty, `D` — собственный протокол DimaSter.
    pub(crate) fn handle_apc(&mut self, data: &[u8]) {
        match data.split_first() {
            Some((b'G', rest)) => self.kitty(rest),
            Some((b'D', rest)) => self.own_protocol(rest),
            _ => log::debug!("неизвестный APC"),
        }
    }

    /// `ESC _ D a=show,c=<колонки>,r=<строки>;<base64 файла> ESC \`:
    /// терминал сам декодирует любой формат, включая анимацию.
    fn own_protocol(&mut self, data: &[u8]) {
        let Some(split) = data.iter().position(|&b| b == b';') else {
            return;
        };
        let header = String::from_utf8_lossy(&data[..split]);
        let value = |key: &str| {
            header
                .split(',')
                .find_map(|p| p.strip_prefix(key)?.strip_prefix('=')?.parse().ok())
        };
        if !header.split(',').any(|p| p == "a=show") {
            return;
        }
        if let Some(animation) = decode_base64(&data[split + 1..]).and_then(|d| decode(&d)) {
            let picture = self.graphics.picture(animation);
            let cells = self.image_cells(&picture, value("c"), value("r"));
            if value("C") == Some(1) {
                self.place(picture, cells, (0, 0));
            } else {
                self.place_and_advance(picture, cells, (0, 0));
            }
        }
    }

    /// DCS `q`: после картинки курсор встаёт в начало следующей строки.
    pub(crate) fn sixel(&mut self, data: &[u8]) {
        let Some(image) = decode_sixel(data) else {
            return;
        };
        let animation = Animation::still(image.width, image.height, image.pixels);
        let picture = self.graphics.picture(animation);
        let cells = self.image_cells(&picture, None, None);
        self.place_and_advance(picture, cells, (0, 0));
        self.set_col(0);
        self.line_feed();
    }
}
