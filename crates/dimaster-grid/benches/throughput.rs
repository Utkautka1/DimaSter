//! Скорость обработки потока — основной показатель «быстрого» терминала.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use dimaster_grid::{Size, Terminal};

fn throughput(c: &mut Criterion) {
    let cases = [
        ("ascii", "The quick brown fox jumps over the lazy dog\r\n"),
        (
            "colors",
            "\x1b[38;2;10;20;30mcolor\x1b[0m \x1b[1mbold\x1b[0m\r\n",
        ),
        ("unicode", "Съешь ещё этих булок 漢字 🚀\r\n"),
    ];
    let mut group = c.benchmark_group("advance");
    for (name, line) in cases {
        let data = line.repeat(10_000).into_bytes();
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_function(name, |b| {
            let mut term = Terminal::new(Size {
                rows: 50,
                cols: 200,
            });
            b.iter(|| term.advance(&data));
        });
    }
    group.finish();
}

/// Reflow полной истории: срабатывает на каждом шаге перетаскивания края окна.
fn reflow(c: &mut Criterion) {
    let mut term = Terminal::new(Size {
        rows: 50,
        cols: 200,
    });
    let line = "The quick brown fox jumps over the lazy dog\r\n";
    term.advance(line.repeat(10_100).as_bytes());
    let mut narrow = false;
    c.bench_function("reflow_10k_lines", |b| {
        b.iter(|| {
            narrow = !narrow;
            let cols = if narrow { 199 } else { 200 };
            term.resize(Size { rows: 50, cols });
        });
    });
}

criterion_group!(benches, throughput, reflow);
criterion_main!(benches);
