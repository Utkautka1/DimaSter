//! Псевдографика DEC: ncurses включает её для рамок через `ESC ( 0`.

/// Наборы G0/G1 и выбранный из них.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Charsets {
    // true — в слоте лежит DEC Special Graphics
    pub(crate) graphics: [bool; 2],
    pub(crate) active: usize,
}

impl Charsets {
    pub(crate) fn map(self, ch: char) -> char {
        // таблица для диапазона '`'..='~'
        const TABLE: &str = "◆▒␉␌␍␊°±␤␋┘┐┌└┼⎺⎻─⎼⎽├┤┴┬│≤≥π≠£·";
        if !self.graphics[self.active] || !('`'..='~').contains(&ch) {
            return ch;
        }
        TABLE.chars().nth(ch as usize - '`' as usize).unwrap_or(ch)
    }
}
