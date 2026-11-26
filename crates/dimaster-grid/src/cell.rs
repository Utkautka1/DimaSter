//! Ячейка сетки и её оформление.

/// Цвет: по умолчанию из темы, из палитры или прямой RGB.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// Битовый набор атрибутов текста.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Flags(u16);

impl Flags {
    pub const BOLD: Self = Self(1);
    pub const DIM: Self = Self(1 << 1);
    pub const ITALIC: Self = Self(1 << 2);
    pub const UNDERLINE: Self = Self(1 << 3);
    pub const BLINK: Self = Self(1 << 4);
    pub const INVERSE: Self = Self(1 << 5);
    pub const HIDDEN: Self = Self(1 << 6);
    pub const STRIKE: Self = Self(1 << 7);
    /// Левая половина широкого символа.
    pub const WIDE: Self = Self(1 << 8);
    /// Правая, пустая половина широкого символа.
    pub const WIDE_TAIL: Self = Self(1 << 9);

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn set(&mut self, other: Self, on: bool) {
        if on {
            self.0 |= other.0;
        } else {
            self.0 &= !other.0;
        }
    }
}

/// Оформление, применяемое к печатаемым символам.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub flags: Flags,
}

/// Одна клетка экрана.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    /// Комбинируемые знаки; Box, потому что встречаются редко и не должны раздувать ячейку.
    pub extra: Option<Box<String>>,
    pub style: Style,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            extra: None,
            style: Style::default(),
        }
    }
}

impl Cell {
    /// Пустая клетка после стирания: сохраняет только фон (поведение BCE).
    pub fn blank(style: Style) -> Self {
        Self {
            style: Style {
                bg: style.bg,
                ..Style::default()
            },
            ..Self::default()
        }
    }

    pub fn new(ch: char, style: Style) -> Self {
        Self {
            ch,
            extra: None,
            style,
        }
    }
}
