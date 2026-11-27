//! Картинки на экране.
//!
//! Картинка привязана к строке, на которой начинается: так она сама
//! прокручивается вместе с текстом, уходит в историю, переживает reflow и
//! освобождается, когда строка вытесняется.

mod budget;
mod defer;
mod diacritics;
mod fit;
mod iterm;
mod kitty;
mod place;
mod store;

use dimaster_i18n::t;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use dimaster_image::{Animation, decode_base64, decode_sixel};
use parking_lot::RwLock;

pub(crate) use defer::Then;
pub use defer::{DecodedImage, SlowImage};
pub use diacritics::{PLACEHOLDER_CHAR, PlaceholderCell, placeholder_info};
pub(crate) use kitty::Pending;
pub(crate) use store::Store;

use crate::terminal::State;

/// Состояние воспроизведения анимации Kitty (`a=a`).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Playback {
    pub(crate) running: bool,
    pub(crate) paused_frame: usize,
    pub(crate) loops: u32,
}

impl Default for Playback {
    fn default() -> Self {
        Self {
            running: true,
            paused_frame: 0,
            loops: 0,
        }
    }
}

/// Декодированная картинка, общая для всех мест, где она показана.
#[derive(Debug)]
pub struct Picture {
    /// Уникальный номер — по нему рендер кэширует текстуры.
    pub serial: u64,
    pub animation: RwLock<Animation>,
    /// Начало отсчёта анимации.
    pub started: RwLock<Instant>,
    pub(crate) generation: AtomicU64,
    pub(crate) playback: RwLock<Playback>,
}

impl Picture {
    pub fn new(serial: u64, animation: Animation) -> Self {
        Self {
            serial,
            animation: RwLock::new(animation),
            started: RwLock::new(Instant::now()),
            generation: AtomicU64::new(0),
            playback: RwLock::new(Playback::default()),
        }
    }

    pub fn dimensions(&self) -> (usize, usize) {
        let anim = self.animation.read();
        (anim.width, anim.height)
    }

    pub fn bytes(&self) -> usize {
        self.animation.read().bytes()
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }

    pub fn current_frame(&self) -> (usize, Option<Duration>) {
        let playback = *self.playback.read();
        let anim = self.animation.read();
        if !playback.running {
            return (
                playback
                    .paused_frame
                    .min(anim.frames.len().saturating_sub(1)),
                None,
            );
        }
        anim.frame_at_limit(self.started.read().elapsed(), playback.loops)
    }

    pub fn with_frame<R>(&self, frame: usize, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
        let anim = self.animation.read();
        anim.frames.get(frame).map(|fr| f(&fr.pixels))
    }
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
    /// Обрезка исходника в пикселях `[x, y, w, h]`.
    pub crop: Option<[u32; 4]>,
}

/// Виртуальное размещение (`U=1`) для Unicode-плейсхолдеров (`U+10EEEE`).
#[derive(Debug, Clone)]
pub struct VirtualPlacement {
    pub picture: Arc<Picture>,
    pub cols: usize,
    pub rows: usize,
}

impl State {
    /// APC: `G` — протокол Kitty, `D` — собственный протокол DimaSter.
    pub(crate) fn handle_apc(&mut self, data: &[u8]) {
        match data.split_first() {
            Some((b'G', rest)) => self.kitty(rest),
            Some((b'D', rest)) => self.own_protocol(rest),
            _ => log::debug!("{}", t!("неизвестный APC")),
        }
    }

    /// `ESC _ D a=show,c=<колонки>,r=<строки>;<base64 файла> ESC \`:
    /// терминал сам декодирует любой формат, включая анимацию.
    fn own_protocol(&mut self, data: &[u8]) {
        let Some(split) = data.iter().position(|&b| b == b';') else {
            return;
        };
        let header = String::from_utf8_lossy(&data[..split]).into_owned();
        if !header.split(',').any(|p| p == "a=show") {
            return;
        }
        if let Some(file) = decode_base64(&data[split + 1..]) {
            self.decode_or_defer(file, Then::Own(header));
        }
    }

    fn show_own(&mut self, header: &str, animation: Option<Animation>) {
        let Some(animation) = animation else { return };
        let value = |key: &str| {
            header
                .split(',')
                .find_map(|p| p.strip_prefix(key)?.strip_prefix('=')?.parse().ok())
        };
        let picture = self.graphics.picture(animation);
        let mode = header.split(',').find_map(|p| p.strip_prefix("fit="));
        let (cells, crop) = self.own_cells(&picture, mode, (value("c"), value("r")));
        if value("C") == Some(1) {
            self.place(picture, cells, (0, 0), crop);
        } else {
            self.place_and_advance(picture, cells, (0, 0), crop);
        }
    }

    // `fit` имеет смысл, только когда рамка задана обеими сторонами
    fn own_cells(
        &self,
        picture: &Picture,
        mode: Option<&str>,
        size: (Option<usize>, Option<usize>),
    ) -> fit::Fitted {
        match (mode, size) {
            (Some(mode), (Some(c), Some(r))) => {
                fit::fit(mode, picture.dimensions(), (c, r), self.cell_size())
            }
            (_, (c, r)) => (self.image_cells(picture, c, r, None), None),
        }
    }

    /// DCS `q`: после картинки курсор встаёт в начало следующей строки.
    pub(crate) fn sixel(&mut self, data: &[u8]) {
        let Some(image) = decode_sixel(data) else {
            return;
        };
        let animation = Animation::still(image.width, image.height, image.pixels);
        let picture = self.graphics.picture(animation);
        let cells = self.image_cells(&picture, None, None, None);
        self.place_and_advance(picture, cells, (0, 0), None);
        self.set_col(0);
        self.line_feed();
    }
}
