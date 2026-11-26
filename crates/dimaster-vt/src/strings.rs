//! Буфер строковых последовательностей OSC, DCS и APC.

// 16 МиБ: картинки передаются частями, но Sixel приходит одним куском.
const MAX_STRING: usize = 16 * 1024 * 1024;

/// Тип открытой строковой последовательности.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StringKind {
    Osc,
    Dcs,
    Apc,
    /// SOS и PM: читаем до терминатора и выбрасываем.
    Ignored,
}

/// Накопленные данные строковой последовательности.
#[derive(Debug)]
pub(crate) struct StringBuf {
    pub(crate) kind: StringKind,
    pub(crate) data: Vec<u8>,
}

impl Default for StringBuf {
    fn default() -> Self {
        Self {
            kind: StringKind::Ignored,
            data: Vec::new(),
        }
    }
}

impl StringBuf {
    pub(crate) fn open(&mut self, kind: StringKind) {
        self.kind = kind;
        self.data.clear();
    }

    pub(crate) fn push(&mut self, byte: u8) {
        if self.kind != StringKind::Ignored && self.data.len() < MAX_STRING {
            self.data.push(byte);
        }
    }
}
