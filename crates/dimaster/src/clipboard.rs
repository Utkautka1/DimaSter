//! Системный буфер обмена и primary selection (вставка средней кнопкой).

use arboard::{GetExtLinux, LinuxClipboardKind, SetExtLinux};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Target {
    Clipboard,
    Primary,
}

impl Target {
    fn kind(self) -> LinuxClipboardKind {
        match self {
            Self::Clipboard => LinuxClipboardKind::Clipboard,
            Self::Primary => LinuxClipboardKind::Primary,
        }
    }
}

/// Буфер обмена; без него терминал продолжает работать.
pub(crate) struct Clipboard(Option<arboard::Clipboard>);

impl Clipboard {
    pub(crate) fn new() -> Self {
        let clipboard =
            arboard::Clipboard::new().inspect_err(|e| log::warn!("Буфер обмена недоступен: {e}"));
        Self(clipboard.ok())
    }

    pub(crate) fn set(&mut self, target: Target, text: String) {
        let Some(clipboard) = &mut self.0 else { return };
        if let Err(e) = clipboard.set().clipboard(target.kind()).text(text) {
            log::warn!("Не удалось скопировать в {target:?}: {e}");
        }
    }

    pub(crate) fn get(&mut self, target: Target) -> Option<String> {
        let clipboard = self.0.as_mut()?;
        clipboard
            .get()
            .clipboard(target.kind())
            .text()
            .inspect_err(|e| log::debug!("в {target:?} нет текста: {e}"))
            .ok()
    }
}
