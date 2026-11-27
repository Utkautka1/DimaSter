//! Графики из колонок текущей таблицы: гистограмма блоками и точки
//! шрифтом Брайля (в знакоместе 2×4 точки).

use dimaster_data::{Database, Rows, Value};
use dimaster_i18n::t;

use crate::render::fitted;
use crate::sheet::Sheet;

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

// есть ли в колонке хоть одно число
fn column(db: &Database, data: &Rows, name: &str) -> Result<usize, String> {
    let found = data
        .columns
        .iter()
        .position(|c| c.eq_ignore_ascii_case(name));
    let numeric = |i: usize| {
        let mut any = false;
        let scanned = db.scan(data, |row| {
            any |= number(&row[i]).is_some();
            Ok(())
        });
        scanned.is_ok() && any
    };
    let found = found.filter(|&i| numeric(i));
    found.ok_or_else(|| t!("Нет числовой колонки: {column}", column = name))
}

/// Разбирает `hist колонка [столбцов]` и `scatter x y`.
pub(crate) fn parse(
    db: &Database,
    data: &Rows,
    word: &str,
    rest: &str,
) -> Option<Result<Plot, String>> {
    let mut args = rest.split_whitespace();
    let (first, second) = (args.next().unwrap_or_default(), args.next());
    Some(match word {
        "hist" => column(db, data, first).map(|column| Plot::Histogram {
            column,
            bins: second.and_then(|n| n.parse().ok()).filter(|&n| n > 0),
        }),
        "scatter" => column(db, data, first)
            .and_then(|x| Ok((x, column(db, data, second.unwrap_or_default())?)))
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

// значения строк, прошедших поиск; хранятся только числа графика
fn collect<T>(db: &Database, sheet: &Sheet, pick: impl Fn(&[Value]) -> Option<T>) -> Vec<T> {
    let mut wanted = vec![sheet.found().is_none(); sheet.data.len()];
    for &index in sheet.found().unwrap_or_default() {
        wanted[index] = true;
    }
    let mut wanted = wanted.into_iter();
    let mut values = Vec::new();
    // ошибка чтения оставит график по прочитанной части
    let _ = db.scan(&sheet.data, |row| {
        if wanted.next() == Some(true) {
            values.extend(pick(row));
        }
        Ok(())
    });
    values
}

/// Строки графика не шире `width` и не выше `height`.
pub(crate) fn draw(
    plot: &Plot,
    db: &Database,
    sheet: &Sheet,
    width: usize,
    height: usize,
) -> Vec<String> {
    let names = &sheet.data.columns;
    let body = height.saturating_sub(2).max(1);
    let (title, lines) = match *plot {
        Plot::Histogram { column, bins } => {
            let values = collect(db, sheet, |r| number(&r[column]));
            let bins = bins.unwrap_or(20).min(body);
            (names[column].clone(), histogram(&values, bins, width))
        }
        Plot::Scatter { x, y } => {
            let points = collect(db, sheet, |r| Some((number(&r[x])?, number(&r[y])?)));
            let title = format!("{} × {}", names[x], names[y]);
            (title, scatter(&points, width.max(1), body))
        }
    };
    let all = std::iter::once(title).chain(lines);
    all.take(height).map(|l| fitted(&l, width)).collect()
}

#[cfg(test)]
mod tests {
    use dimaster_data::{Format, Options};

    use super::*;

    fn table() -> (Database, Rows) {
        let mut db = Database::new().expect("база");
        let text = b"x,name,y\n1,a,1\n2,b,4\n9,c,9\n";
        db.load_bytes("t", Format::Csv, text, &Options::default())
            .expect("csv");
        let data = db.open_table("t").expect("таблица");
        (db, data)
    }

    #[test]
    fn only_numeric_columns_are_plotted() {
        let (db, data) = table();
        let scatter = parse(&db, &data, "scatter", "X y");
        assert_eq!(scatter, Some(Ok(Plot::Scatter { x: 0, y: 2 })));
        assert!(matches!(parse(&db, &data, "hist", "name"), Some(Err(_))));
        assert_eq!(parse(&db, &data, "order", "x"), None);
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
