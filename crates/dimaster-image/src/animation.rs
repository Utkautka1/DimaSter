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
    /// Второе значение `None` у неподвижных картинок.
    pub fn frame_at(&self, elapsed: Duration) -> (usize, Option<Duration>) {
        let total: u64 = self.frames.iter().map(|f| u64::from(f.delay)).sum();
        if self.frames.len() < 2 || total == 0 {
            return (0, None);
        }
        let mut position = (elapsed.as_millis() % u128::from(total)) as u64;
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

/// Определяет формат по сигнатуре и декодирует.
pub fn decode(data: &[u8]) -> Option<Animation> {
    if data.starts_with(b"\x89PNG") {
        // редкие варианты PNG (16 бит, интерлейс) достаются ffmpeg
        return match decode_png(data) {
            Some(image) => Some(Animation::still(image.width, image.height, image.pixels)),
            None => external::decode(data),
        };
    }
    if data.starts_with(b"GIF8") {
        return decode_gif(data);
    }
    external::decode(data)
}
