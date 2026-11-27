//! Графики из колонок текущей таблицы: гистограмма блоками и точки
//! шрифтом Брайля (в знакоместе 2×4 точки).

use dimaster_data::{Table, Value};
use dimaster_i18n::t;

use crate::render::fitted;

/// Что нарисовать; номера — колонки результата.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Plot {
    Histogram { column: usize, bins: Option<usize> },
    Scatter { x: usize, y: usize },
}

fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Int(n) => Some(*n as f64),
        Value::Real(n) => Some(*n),
        _ => None,
    }
}

fn column(table: &Table, name: &str) -> Result<usize, String> {
    let found = table
        .columns
        .iter()
        .position(|c| c.eq_ignore_ascii_case(name));
    let found = found.filter(|&i| table.rows.iter().any(|r| number(&r[i]).is_some()));
    found.ok_or_else(|| t!("Нет числовой колонки: {column}", column = name))
}

/// Разбирает `hist колонка [столбцов]` и `scatter x y`.
pub(crate) fn parse(table: &Table, word: &str, rest: &str) -> Option<Result<Plot, String>> {
    let mut args = rest.split_whitespace();
    let (first, second) = (args.next().unwrap_or_default(), args.next());
    Some(match word {
        "hist" => column(table, first).map(|column| Plot::Histogram {
            column,
            bins: second.and_then(|n| n.parse().ok()).filter(|&n| n > 0),
        }),
        "scatter" => column(table, first)
            .and_then(|x| Ok((x, column(table, second.unwrap_or_default())?)))
            .map(|(x, y)| Plot::Scatter { x, y }),
        _ => return None,
    })
}

// наименьшее и наибольшее; отрезок никогда не бывает пустым
fn bounds(values: impl Iterator<Item = f64>) -> (f64, f64) {
    let (low, high) = values.fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(v), h.max(v)));
    match high > low {
        true => (low, high),
        false => (low, low + 1.0),
    }
}

// номер деления для значения на отрезке из `count` делений
fn place(value: f64, (low, high): (f64, f64), count: usize) -> usize {
    let share = (value - low) / (high - low);
    ((share * count as f64) as usize).min(count - 1)
}

fn bar(count: usize, most: usize, width: usize) -> String {
    const PARTS: [&str; 8] = ["", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];
    let eighths = count * width * 8 / most.max(1);
    "█".repeat(eighths / 8) + PARTS[eighths % 8]
}

fn histogram(values: &[f64], bins: usize, width: usize) -> Vec<String> {
    let range = bounds(values.iter().copied());
    let mut counts = vec![0usize; bins];
    for &value in values {
        counts[place(value, range, bins)] += 1;
    }
    let most = counts.iter().copied().max().unwrap_or(0);
    let step = (range.1 - range.0) / bins as f64;
    let line = |(i, &count): (usize, &usize)| {
        let label = format!("{:>12.2} {count:>7} ", range.0 + step * i as f64);
        let room = width.saturating_sub(label.len());
        label + &bar(count, most, room)
    };
    counts.iter().enumerate().map(line).collect()
}

// биты точек Брайля по строке и столбцу внутри знакоместа
const DOTS: [[u32; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

fn scatter(points: &[(f64, f64)], width: usize, height: usize) -> Vec<String> {
    let across = bounds(points.iter().map(|p| p.0));
    let up = bounds(points.iter().map(|p| p.1));
    let mut cells = vec![vec![0u32; width]; height];
    for &(x, y) in points {
        let column = place(x, across, width * 2);
        let row = height * 4 - 1 - place(y, up, height * 4);
        cells[row / 4][column / 2] |= DOTS[row % 4][column % 2];
    }
    let letter = |bits: &u32| char::from_u32(0x2800 + bits).unwrap_or(' ');
    let mut lines: Vec<String> = cells
        .iter()
        .map(|r| r.iter().map(letter).collect())
        .collect();
    lines.push(format!(
        "x: {} … {}   y: {} … {}",
        across.0, across.1, up.0, up.1
    ));
    lines
}

/// Строки графика не шире `width` и не выше `height`.
pub(crate) fn draw(
    plot: &Plot,
    table: &Table,
    shown: &[usize],
    width: usize,
    height: usize,
) -> Vec<String> {
    let rows = shown.iter().map(|&i| &table.rows[i]);
    let body = height.saturating_sub(2).max(1);
    let (title, lines) = match *plot {
        Plot::Histogram { column, bins } => {
            let values: Vec<f64> = rows.filter_map(|r| number(&r[column])).collect();
            let bins = bins.unwrap_or(20).min(body);
            (
                table.columns[column].clone(),
                histogram(&values, bins, width),
            )
        }
        Plot::Scatter { x, y } => {
            let points: Vec<_> = rows
                .filter_map(|r| Some((number(&r[x])?, number(&r[y])?)))
                .collect();
            let title = format!("{} × {}", table.columns[x], table.columns[y]);
            (title, scatter(&points, width.max(1), body))
        }
    };
    let all = std::iter::once(title).chain(lines);
    all.take(height).map(|l| fitted(&l, width)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Table {
        let rows = [["1", "a", "1"], ["2", "b", "4"], ["9", "c", "9"]];
        let rows = rows.iter().map(|r| r.map(String::from).to_vec()).collect();
        Table::from_text(["x", "name", "y"].map(String::from).to_vec(), rows)
    }

    #[test]
    fn only_numeric_columns_are_plotted() {
        let table = table();
        let scatter = parse(&table, "scatter", "X y");
        assert_eq!(scatter, Some(Ok(Plot::Scatter { x: 0, y: 2 })));
        assert!(matches!(parse(&table, "hist", "name"), Some(Err(_))));
        assert_eq!(parse(&table, "order", "x"), None);
    }

    #[test]
    fn histogram_counts_and_scatter_corners() {
        let lines = histogram(&[0.0, 0.0, 10.0], 2, 40);
        assert!(lines[0].contains(" 2 ") && lines[1].contains(" 1 "));
        let lines = scatter(&[(0.0, 0.0), (1.0, 1.0)], 4, 2);
        assert_eq!(lines[0].chars().last(), Some('⠈'));
        assert_eq!(lines[1].chars().next(), Some('⡀'));
    }
}
