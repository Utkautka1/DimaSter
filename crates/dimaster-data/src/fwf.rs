//! Колонки фиксированной ширины (вывод `ps`, `df` и подобный): граница — там,
//! где пробел стоит во всех строках сразу.

use crate::Table;

// начала и концы колонок в символах
fn spans(lines: &[Vec<char>]) -> Vec<(usize, usize)> {
    let width = lines.iter().map(Vec::len).max().unwrap_or(0);
    let used = |at: usize| {
        lines
            .iter()
            .any(|l| l.get(at).is_some_and(|c| !c.is_whitespace()))
    };
    let mut spans = Vec::new();
    let mut start = None;
    for at in 0..=width {
        match (start, at < width && used(at)) {
            (None, true) => start = Some(at),
            (Some(from), false) => {
                spans.push((from, at));
                start = None;
            }
            _ => {}
        }
    }
    spans
}

fn cut(line: &[char], span: (usize, usize)) -> String {
    let end = span.1.min(line.len());
    let part = &line[span.0.min(end)..end];
    part.iter().collect::<String>().trim().to_owned()
}

pub(crate) fn parse(text: &str, header: bool) -> Table {
    let lines: Vec<Vec<char>> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.chars().collect())
        .collect();
    let spans = spans(&lines);
    let mut rows: Vec<Vec<String>> = lines
        .iter()
        .map(|line| spans.iter().map(|&s| cut(line, s)).collect())
        .collect();
    let columns = match header && !rows.is_empty() {
        true => rows.remove(0),
        false => Vec::new(),
    };
    Table::from_text(columns, rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn splits_on_common_blanks() {
        let table = parse("name     size\nmy disk    10\nb         200\n", true);
        assert_eq!(table.columns, ["name", "size"]);
        assert_eq!(
            table.rows[0],
            [Value::Text("my disk".into()), Value::Int(10)]
        );
    }
}
