//! Запросы настроек и возможностей (DECRQSS, XTGETTCAP) и мягкий сброс.

use crate::terminal::State;
use crate::{Color, CursorShape, Flags, Style};

// возможности, о которых спрашивают nvim и tmux
const CAPABILITIES: [(&str, &str); 5] = [
    ("TN", "xterm-dimaster"),
    ("Co", "256"),
    ("colors", "256"),
    ("RGB", "8"),
    ("Ms", "\x1b]52;%p1%s;%p2%s\x07"),
];

fn hex(text: &str) -> String {
    text.bytes().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &[u8]) -> Option<String> {
    let pairs = text.chunks(2).map(|pair| {
        let digits = std::str::from_utf8(pair).ok().filter(|d| d.len() == 2)?;
        u8::from_str_radix(digits, 16).ok()
    });
    String::from_utf8(pairs.collect::<Option<Vec<u8>>>()?).ok()
}

// параметры SGR для одного цвета; `base` — 30 для текста, 40 для фона
fn color_codes(color: Color, base: u8) -> Option<String> {
    match color {
        Color::Default => None,
        Color::Indexed(i) => Some(format!("{}:5:{i}", base + 8)),
        Color::Rgb(r, g, b) => Some(format!("{}:2::{r}:{g}:{b}", base + 8)),
    }
}

fn sgr(style: Style) -> String {
    let flags = [
        (Flags::BOLD, "1"),
        (Flags::DIM, "2"),
        (Flags::ITALIC, "3"),
        (Flags::UNDERLINE, "4"),
        (Flags::BLINK, "5"),
        (Flags::INVERSE, "7"),
        (Flags::HIDDEN, "8"),
        (Flags::STRIKE, "9"),
    ];
    let mut codes = vec!["0".to_owned()];
    let active = flags.iter().filter(|(f, _)| style.flags.contains(*f));
    codes.extend(active.map(|(_, code)| (*code).to_owned()));
    codes.extend(color_codes(style.fg, 30));
    codes.extend(color_codes(style.bg, 40));
    codes.join(";")
}

impl State {
    /// DECRQSS: текущее значение настройки.
    pub(crate) fn setting_report(&mut self, name: &[u8]) {
        let shape = match self.modes.cursor_shape {
            CursorShape::Block => 2,
            CursorShape::Underline => 4,
            CursorShape::Bar => 6,
        };
        let value = match name {
            b"m" => Some(format!("{}m", sgr(self.cursor.style))),
            b"r" => Some(format!(
                "{};{}r",
                self.screen.top + 1,
                self.screen.bottom + 1
            )),
            b" q" => Some(format!("{shape} q")),
            _ => None,
        };
        match value {
            Some(value) => self.answer(format!("\x1bP1$r{value}\x1b\\")),
            None => self.answer("\x1bP0$r\x1b\\".to_owned()),
        }
    }

    /// XTGETTCAP: имена и значения передаются в шестнадцатеричном виде.
    ///
    /// Ответ уходит в программу как её ввод, поэтому имя повторяется, только
    /// если это действительно шестнадцатеричные цифры: иначе вывод файла мог
    /// бы «напечатать» команду в shell.
    pub(crate) fn capability_report(&mut self, request: &[u8]) {
        for name in request.split(|&b| b == b';').take(32) {
            let hexadecimal = name.len() <= 64 && name.iter().all(u8::is_ascii_hexdigit);
            let Some(decoded) = unhex(name).filter(|_| hexadecimal) else {
                self.answer("\x1bP0+r\x1b\\".to_owned());
                continue;
            };
            let raw = String::from_utf8_lossy(name);
            match CAPABILITIES.iter().find(|c| c.0 == decoded) {
                Some((_, value)) => self.answer(format!("\x1bP1+r{raw}={}\x1b\\", hex(value))),
                None => self.answer(format!("\x1bP0+r{raw}\x1b\\")),
            }
        }
    }

    /// DECSTR: режимы и оформление по умолчанию, экран не очищается.
    pub(crate) fn soft_reset(&mut self) {
        self.modes.insert = false;
        self.modes.origin = false;
        self.modes.autowrap = true;
        self.modes.cursor_visible = true;
        self.modes.app_cursor = false;
        self.modes.app_keypad = false;
        self.cursor.style = Style::default();
        self.cursor.pending_wrap = false;
        self.screen.top = 0;
        self.screen.bottom = self.screen.rows.len() - 1;
        self.saved = Default::default();
    }
}
