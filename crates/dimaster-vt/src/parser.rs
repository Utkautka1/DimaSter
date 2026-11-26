//! Конечный автомат разбора. Состояние живёт между вызовами [`Parser::advance`].

use crate::strings::{StringBuf, StringKind};
use crate::utf8::{Decoded, Utf8};
use crate::{Params, Perform, Sequence};

const ESC: u8 = 0x1B;
const BEL: u8 = 0x07;
// CAN и SUB прерывают любую незаконченную последовательность
const CAN: u8 = 0x18;
const SUB: u8 = 0x1A;
const MAX_INTERMEDIATES: usize = 4;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum State {
    #[default]
    Ground,
    Escape,
    Csi,
    /// Параметры DCS до финального байта.
    DcsEntry,
    /// Сбор тела OSC/DCS/APC.
    Str,
    /// ESC внутри строки: ждём `\` (терминатор ST).
    StrEscape,
}

/// Потоковый разборщик вывода программ.
#[derive(Debug, Default)]
pub struct Parser {
    state: State,
    params: Params,
    intermediates: Vec<u8>,
    // финальный байт DCS, запомненный до конца строки
    dcs_action: u8,
    string: StringBuf,
    utf8: Utf8,
}

impl Parser {
    /// Разбирает очередную порцию байтов; порции можно резать где угодно.
    pub fn advance<P: Perform>(&mut self, perform: &mut P, bytes: &[u8]) {
        for &byte in bytes {
            self.step(perform, byte);
        }
    }

    fn step<P: Perform>(&mut self, p: &mut P, byte: u8) {
        match self.state {
            State::Str => self.in_string(p, byte),
            State::StrEscape => self.string_escape(p, byte),
            _ if self.utf8.is_pending() => self.decode(p, byte),
            _ => self.control_or_sequence(p, byte),
        }
    }

    fn control_or_sequence<P: Perform>(&mut self, p: &mut P, byte: u8) {
        match byte {
            ESC => self.enter_escape(),
            CAN | SUB => self.state = State::Ground,
            // C0 исполняются даже посреди CSI — так делает xterm
            0x00..=0x1F => p.execute(byte),
            0x7F => {}
            _ => match self.state {
                State::Ground => self.ground(p, byte),
                State::Escape => self.escape(p, byte),
                _ => self.sequence(p, byte),
            },
        }
    }

    fn ground<P: Perform>(&mut self, p: &mut P, byte: u8) {
        if byte < 0x80 {
            p.print(char::from(byte));
        } else {
            self.decode(p, byte);
        }
    }

    fn decode<P: Perform>(&mut self, p: &mut P, byte: u8) {
        match self.utf8.push(byte) {
            Decoded::Pending => {}
            Decoded::Char(ch) => p.print(ch),
            Decoded::Retry => {
                p.print(char::REPLACEMENT_CHARACTER);
                self.step(p, byte);
            }
        }
    }

    fn enter_escape(&mut self) {
        self.state = State::Escape;
        self.params.clear();
        self.intermediates.clear();
    }

    fn escape<P: Perform>(&mut self, p: &mut P, byte: u8) {
        match byte {
            b'[' => self.state = State::Csi,
            b']' => self.open_string(StringKind::Osc),
            b'P' => self.state = State::DcsEntry,
            b'_' => self.open_string(StringKind::Apc),
            b'X' | b'^' => self.open_string(StringKind::Ignored),
            0x20..=0x2F => self.collect(byte),
            0x30..=0x7E => {
                self.state = State::Ground;
                p.esc(self.sequence_view(byte));
            }
            _ => self.state = State::Ground,
        }
    }

    /// Общая часть CSI и DCS: параметры, префиксы, финальный байт.
    fn sequence<P: Perform>(&mut self, p: &mut P, byte: u8) {
        match byte {
            b'0'..=b';' => self.params.feed(byte),
            0x20..=0x2F | 0x3C..=0x3F => self.collect(byte),
            0x40..=0x7E => self.finish_sequence(p, byte),
            _ => self.state = State::Ground,
        }
    }

    fn finish_sequence<P: Perform>(&mut self, p: &mut P, byte: u8) {
        self.params.finish();
        if self.state == State::DcsEntry {
            self.dcs_action = byte;
            self.open_string(StringKind::Dcs);
        } else {
            self.state = State::Ground;
            p.csi(self.sequence_view(byte));
        }
    }

    fn sequence_view(&self, action: u8) -> Sequence<'_> {
        Sequence {
            params: &self.params,
            intermediates: &self.intermediates,
            action,
        }
    }

    fn collect(&mut self, byte: u8) {
        if self.intermediates.len() < MAX_INTERMEDIATES {
            self.intermediates.push(byte);
        }
    }

    fn open_string(&mut self, kind: StringKind) {
        self.string.open(kind);
        self.state = State::Str;
    }

    fn in_string<P: Perform>(&mut self, p: &mut P, byte: u8) {
        match byte {
            ESC => self.state = State::StrEscape,
            BEL if self.string.kind == StringKind::Osc => self.close_string(p),
            CAN | SUB => self.state = State::Ground,
            _ => self.string.push(byte),
        }
    }

    // Любой байт кроме `\` отменяет строку и начинает новую escape-команду
    fn string_escape<P: Perform>(&mut self, p: &mut P, byte: u8) {
        if byte == b'\\' {
            self.close_string(p);
        } else {
            self.enter_escape();
            self.step(p, byte);
        }
    }

    fn close_string<P: Perform>(&mut self, p: &mut P) {
        self.state = State::Ground;
        match self.string.kind {
            StringKind::Osc => p.osc(&self.string.data),
            StringKind::Dcs => p.dcs(self.sequence_view(self.dcs_action), &self.string.data),
            StringKind::Apc => p.apc(&self.string.data),
            StringKind::Ignored => {}
        }
    }
}
