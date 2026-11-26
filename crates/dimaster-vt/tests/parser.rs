//! Проверяет разбор последовательностей, включая разрывы между чтениями.

use dimaster_vt::{Parser, Perform, Sequence};
use proptest::prelude::*;

/// Записывает команды строками, чтобы сравнивать их напрямую.
#[derive(Default, PartialEq, Debug)]
struct Log(Vec<String>);

impl Log {
    fn seq(&mut self, kind: &str, s: &Sequence<'_>) {
        let params: Vec<_> = s.params.groups().collect();
        let inter = String::from_utf8_lossy(s.intermediates).into_owned();
        self.0
            .push(format!("{kind} {inter}{params:?}{}", char::from(s.action)));
    }
}

impl Perform for Log {
    fn print(&mut self, ch: char) {
        self.0.push(format!("print {ch}"));
    }
    fn execute(&mut self, byte: u8) {
        self.0.push(format!("exec {byte}"));
    }
    fn esc(&mut self, seq: Sequence<'_>) {
        self.seq("esc", &seq);
    }
    fn csi(&mut self, seq: Sequence<'_>) {
        self.seq("csi", &seq);
    }
    fn osc(&mut self, data: &[u8]) {
        self.0
            .push(format!("osc {}", String::from_utf8_lossy(data)));
    }
    fn dcs(&mut self, seq: Sequence<'_>, data: &[u8]) {
        self.seq("dcs", &seq);
        self.osc(data);
    }
    fn apc(&mut self, data: &[u8]) {
        self.0
            .push(format!("apc {}", String::from_utf8_lossy(data)));
    }
}

fn parse(chunks: &[&[u8]]) -> Vec<String> {
    let (mut parser, mut log) = (Parser::default(), Log::default());
    for chunk in chunks {
        parser.advance(&mut log, chunk);
    }
    log.0
}

#[test]
fn csi_with_prefix_and_subparams() {
    assert_eq!(parse(&[b"\x1b[?1049h"]), ["csi ?[[1049]]h"]);
    assert_eq!(
        parse(&[b"\x1b[38:2:1:2:3;1m"]),
        ["csi [[38, 2, 1, 2, 3], [1]]m"]
    );
    assert_eq!(parse(&[b"\x1b[;5H"]), ["csi [[0], [5]]H"]);
}

#[test]
fn strings_end_with_st_or_bel() {
    assert_eq!(parse(&[b"\x1b]0;title\x07"]), ["osc 0;title"]);
    assert_eq!(parse(&[b"\x1b_Ga=T;AAAA\x1b\\"]), ["apc Ga=T;AAAA"]);
    assert_eq!(parse(&[b"\x1bP1qdata\x1b\\"]), ["dcs [[1]]q", "osc data"]);
}

#[test]
fn utf8_split_between_reads() {
    let bytes = "ё€".as_bytes();
    assert_eq!(
        parse(&[&bytes[..1], &bytes[1..3], &bytes[3..]]),
        ["print ё", "print €"]
    );
}

#[test]
fn broken_utf8_does_not_eat_next_byte() {
    assert_eq!(parse(&[b"\xd0A"]), ["print \u{fffd}", "print A"]);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// Мусор не роняет парсер, а разрез потока не меняет результат.
    #[test]
    fn chunking_is_invisible(data in proptest::collection::vec(any::<u8>(), 0..512), at in 0usize..512) {
        let at = at.min(data.len());
        prop_assert_eq!(parse(&[&data]), parse(&[&data[..at], &data[at..]]));
    }
}
