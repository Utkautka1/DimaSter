//! Числовые параметры CSI/DCS с поддержкой подпараметров через `:`.

// Лимит защищает от бесконечного роста памяти на мусорном вводе.
const MAX_VALUES: usize = 32;

/// Список параметров; каждый параметр — группа из значения и подпараметров.
#[derive(Debug, Default)]
pub struct Params {
    values: Vec<u16>,
    // starts[i] — индекс в values, с которого начинается i-я группа
    starts: Vec<usize>,
    current: u16,
    sub: bool,
    touched: bool,
}

impl Params {
    /// Группы по порядку: `38:2:1;4` даёт `[38,2,1]` и `[4]`.
    pub fn groups(&self) -> impl Iterator<Item = &[u16]> {
        (0..self.starts.len()).map(|i| {
            let end = self.starts.get(i + 1).copied().unwrap_or(self.values.len());
            &self.values[self.starts[i]..end]
        })
    }

    /// Значение i-го параметра; 0 и отсутствие заменяются на `default`.
    pub fn get_or(&self, index: usize, default: u16) -> u16 {
        match self.groups().nth(index).and_then(|g| g.first()) {
            Some(&v) if v != 0 => v,
            _ => default,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.values.clear();
        self.starts.clear();
        self.current = 0;
        self.sub = false;
        self.touched = false;
    }

    /// Принимает цифру, `;` или `:`.
    pub(crate) fn feed(&mut self, byte: u8) {
        self.touched = true;
        match byte {
            b';' => self.commit(false),
            b':' => self.commit(true),
            _ => {
                let digit = u16::from(byte - b'0');
                self.current = self.current.saturating_mul(10).saturating_add(digit);
            }
        }
    }

    /// Закрывает последний параметр перед диспетчеризацией.
    pub(crate) fn finish(&mut self) {
        if self.touched {
            self.commit(false);
        }
    }

    fn commit(&mut self, next_is_sub: bool) {
        if self.values.len() < MAX_VALUES {
            if !self.sub {
                self.starts.push(self.values.len());
            }
            self.values.push(self.current);
        }
        self.current = 0;
        self.sub = next_is_sub;
    }
}
