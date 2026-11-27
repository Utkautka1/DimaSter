//! Картинки, которые декодирует `ffmpeg`.
//!
//! Ждать его с захваченным терминалом нельзя — замёрзло бы окно. Разбор
//! останавливается на такой картинке, поток вывода декодирует её без
//! блокировки и возвращает результат; порядок вывода при этом сохраняется.

use dimaster_image::{Animation, Decoding, decode_external, decode_native};

use super::Pending;
use crate::terminal::State;

/// Что сделать с картинкой после декодирования.
#[derive(Debug)]
pub(crate) enum Then {
    /// Собственный протокол и его заголовок.
    Own(String),
    /// iTerm2 и его заголовок.
    Iterm(String),
    Kitty(Pending),
}

/// Картинка, ждущая внешнего декодера.
#[derive(Debug)]
pub struct SlowImage {
    data: Vec<u8>,
    then: Then,
}

/// Результат [`SlowImage::decode`] для [`crate::Terminal::finish_image`].
#[derive(Debug)]
pub struct DecodedImage {
    animation: Option<Animation>,
    then: Then,
}

impl SlowImage {
    /// Долгий вызов: выполняется без блокировки терминала.
    pub fn decode(self) -> DecodedImage {
        DecodedImage {
            animation: decode_external(&self.data),
            then: self.then,
        }
    }
}

impl State {
    pub(crate) fn decode_or_defer(&mut self, data: Vec<u8>, then: Then) {
        match decode_native(&data) {
            Decoding::Ready(animation) => self.show_decoded(then, animation),
            Decoding::External => self.deferred = Some(SlowImage { data, then }),
        }
    }

    pub(crate) fn finish_image(&mut self, image: DecodedImage) {
        self.show_decoded(image.then, image.animation);
    }

    fn show_decoded(&mut self, then: Then, animation: Option<Animation>) {
        match then {
            Then::Own(header) => self.show_own(&header, animation),
            Then::Iterm(header) => self.show_iterm(&header, animation),
            Then::Kitty(command) => self.kitty_finish(&command, animation),
        }
    }
}
