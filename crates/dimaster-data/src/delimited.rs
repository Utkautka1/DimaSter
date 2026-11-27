//! CSV, TSV и другие файлы с разделителем.

use crate::{DataError, Table};

fn record(record: &csv::StringRecord) -> Vec<String> {
    record.iter().map(str::to_owned).collect()
}

pub(crate) fn parse(text: &str, separator: u8, header: bool) -> Result<Table, DataError> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(separator)
        .has_headers(false)
        // строки разной длины не повод отказываться от всего файла
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut rows = Vec::new();
    for row in reader.records() {
        rows.push(record(&row?));
    }
    let columns = match header && !rows.is_empty() {
        true => rows.remove(0),
        false => Vec::new(),
    };
    Ok(Table::from_text(columns, rows))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn quotes_and_short_rows() {
        let table = parse("a,b\n\"x,y\",2\nz\n", b',', true).expect("csv");
        assert_eq!(table.rows[0], [Value::Text("x,y".into()), Value::Int(2)]);
        assert_eq!(table.rows[1][1], Value::Null);
    }
}
