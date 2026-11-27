//! Контракт между парсером и эмулятором, чтобы их можно было тестировать порознь.

use crate::Params;

/// Разобранная управляющая последовательность CSI, ESC или DCS.
#[derive(Debug)]
pub struct Sequence<'a> {
    pub params: &'a Params,
    /// Приватные префиксы (`?`, `>`, `=`) и промежуточные байты по порядку.
    pub intermediates: &'a [u8],
    /// Финальный байт, определяющий команду.
    pub action: u8,
}

/// Получатель команд парсера. Все методы по умолчанию пустые.
pub trait Perform {
    /// `true` — остановить разбор после только что выполненной команды.
    fn paused(&self) -> bool {
        false
    }
    fn print(&mut self, _ch: char) {}
    /// Серия печатных ASCII-символов подряд — основной поток вывода,
    /// получатель может записать её целиком.
    fn print_ascii(&mut self, run: &[u8]) {
        run.iter().for_each(|&b| self.print(char::from(b)));
    }
    /// Управляющий байт C0 (`\n`, `\r`, BEL и т.п.).
    fn execute(&mut self, _byte: u8) {}
    fn esc(&mut self, _seq: Sequence<'_>) {}
    fn csi(&mut self, _seq: Sequence<'_>) {}
    /// Содержимое OSC без обрамления; разбиение по `;` — дело получателя.
    fn osc(&mut self, _data: &[u8]) {}
    fn dcs(&mut self, _seq: Sequence<'_>, _data: &[u8]) {}
    fn apc(&mut self, _data: &[u8]) {}
}
