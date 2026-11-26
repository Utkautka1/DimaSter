//! Потоковый декодер UTF-8: символ может быть разорван между чтениями из PTY.

/// Накопитель байтов одного символа.
#[derive(Debug, Default)]
pub(crate) struct Utf8 {
    buf: [u8; 4],
    len: usize,
    need: usize,
}

/// Результат приёма очередного байта.
pub(crate) enum Decoded {
    Pending,
    Char(char),
    /// Последовательность оборвана; байт надо разобрать заново.
    Retry,
}

impl Utf8 {
    pub(crate) fn is_pending(&self) -> bool {
        self.need != 0
    }

    pub(crate) fn push(&mut self, byte: u8) -> Decoded {
        if self.need == 0 {
            return self.start(byte);
        }
        if byte & 0xC0 != 0x80 {
            self.need = 0;
            return Decoded::Retry;
        }
        self.buf[self.len] = byte;
        self.len += 1;
        if self.len < self.need {
            return Decoded::Pending;
        }
        self.take()
    }

    fn start(&mut self, byte: u8) -> Decoded {
        self.need = match byte {
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => return Decoded::Char(char::REPLACEMENT_CHARACTER),
        };
        self.buf[0] = byte;
        self.len = 1;
        Decoded::Pending
    }

    // from_utf8 дополнительно отсекает суррогаты и overlong-формы
    fn take(&mut self) -> Decoded {
        self.need = 0;
        let ch = std::str::from_utf8(&self.buf[..self.len])
            .ok()
            .and_then(|s| s.chars().next());
        Decoded::Char(ch.unwrap_or(char::REPLACEMENT_CHARACTER))
    }
}
