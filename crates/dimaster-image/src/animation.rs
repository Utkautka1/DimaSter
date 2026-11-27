//! Общий результат декодирования: неподвижная картинка — это один кадр.

use std::time::Duration;

use crate::{decode_gif, decode_png, external};

/// Кадр в RGBA и время его показа.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub pixels: Vec<u8>,
    // миллисекунды; для неподвижной картинки 0
    pub delay: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animation {
    pub width: usize,
    pub height: usize,
    pub frames: Vec<Frame>,
}

impl Animation {
    pub fn still(width: usize, height: usize, pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            frames: vec![Frame { pixels, delay: 0 }],
        }
    }

    /// Объём пикселей всех кадров в байтах.
    pub fn bytes(&self) -> usize {
        self.frames.iter().map(|f| f.pixels.len()).sum()
    }

    /// Кадр в момент `elapsed` и сколько ещё он продержится.
    /// Второе значение `None` у неподвижных картинок или после окончания циклов.
    pub fn frame_at(&self, elapsed: Duration) -> (usize, Option<Duration>) {
        self.frame_at_limit(elapsed, 0)
    }

    /// Как `frame_at`, но с ограничением числа повторов (`0` — бесконечно).
    pub fn frame_at_limit(&self, elapsed: Duration, loops: u32) -> (usize, Option<Duration>) {
        let total: u64 = self.frames.iter().map(|f| u64::from(f.delay)).sum();
        if self.frames.len() < 2 || total == 0 {
            return (0, None);
        }
        let elapsed_ms = elapsed.as_millis();
        if loops > 0 && elapsed_ms >= u128::from(total) * u128::from(loops) {
            return (self.frames.len() - 1, None);
        }
        let mut position = (elapsed_ms % u128::from(total)) as u64;
        for (index, frame) in self.frames.iter().enumerate() {
            let delay = u64::from(frame.delay);
            if position < delay {
                return (index, Some(Duration::from_millis(delay - position)));
            }
            position -= delay;
        }
        (0, None)
    }
}

/// Итог декодирования своими силами.
#[derive(Debug)]
pub enum Decoding {
    Ready(Option<Animation>),
    /// Нужен внешний `ffmpeg`: это долго.
    External,
}

/// Разбирает PNG и GIF, не запуская внешних программ.
pub fn decode_native(data: &[u8]) -> Decoding {
    if data.starts_with(b"\x89PNG") {
        // редкие варианты PNG (16 бит, интерлейс) достаются ffmpeg
        return match decode_png(data) {
            Some(image) => Decoding::Ready(Some(Animation::still(
                image.width,
                image.height,
                image.pixels,
            ))),
            None => Decoding::External,
        };
    }
    if data.starts_with(b"GIF8") {
        return Decoding::Ready(decode_gif(data));
    }
    Decoding::External
}

/// Декодирует через `ffmpeg`; ждёт его до нескольких секунд.
pub fn decode_external(data: &[u8]) -> Option<Animation> {
    external::decode(data)
}

/// Определяет формат по сигнатуре и декодирует.
pub fn decode(data: &[u8]) -> Option<Animation> {
    match decode_native(data) {
        Decoding::Ready(animation) => animation,
        Decoding::External => decode_external(data),
    }
}
