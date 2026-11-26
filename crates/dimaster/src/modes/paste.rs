//! Подтверждение вставки потенциально опасного текста.

use super::Press;

/// Ожидающая подтверждения вставка из буфера обмена.
#[derive(Debug, Clone)]
pub(crate) struct PasteConfirm {
    pub(crate) text: String,
    pub(crate) reason: String,
}

/// Решение пользователя по опасной вставке.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Stay,
    PasteRaw,
    PasteSingleLine,
    Cancel,
}

impl PasteConfirm {
    pub(crate) fn press(&self, press: Press) -> Outcome {
        match press {
            Press::Enter { .. } => Outcome::PasteRaw,
            Press::Char(' ') => Outcome::PasteSingleLine,
            Press::Escape | Press::Ctrl('c') | Press::Char('q') => Outcome::Cancel,
            _ => Outcome::Stay,
        }
    }
}
