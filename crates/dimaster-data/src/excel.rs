//! Книги Excel и OpenDocument: каждый лист — отдельная таблица.

use std::io::Cursor;

use calamine::{Data, Reader, open_workbook_auto_from_rs};

use crate::{DataError, Table, Value};

fn failed(error: impl std::fmt::Display) -> DataError {
    DataError::Parse(error.to_string())
}

fn value(cell: &Data) -> Value {
    match cell {
        Data::Empty => Value::Null,
        Data::Int(number) => Value::Int(*number),
        // Excel хранит целые как дробные: 3.0 показываем как 3
        Data::Float(number) if number.fract() == 0.0 && number.abs() < 1e15 => {
            Value::Int(*number as i64)
        }
        Data::Float(number) => Value::Real(*number),
        Data::DateTime(moment) => match moment.as_datetime() {
            Some(date) => Value::Text(date.to_string()),
            None => Value::Real(moment.as_f64()),
        },
        other => Value::Text(other.to_string()),
    }
}

fn sheet(name: String, range: &calamine::Range<Data>, header: bool) -> Table {
    let mut rows: Vec<Vec<Value>> = range
        .rows()
        .map(|r| r.iter().map(value).collect())
        .collect();
    let columns = match header && !rows.is_empty() {
        true => rows.remove(0).iter().map(Value::text).collect(),
        false => Vec::new(),
    };
    let mut table = Table::new(columns, rows);
    table.name = name;
    table
}

pub(crate) fn parse(bytes: &[u8], header: bool) -> Result<Vec<Table>, DataError> {
    let mut book = open_workbook_auto_from_rs(Cursor::new(bytes)).map_err(failed)?;
    let mut tables = Vec::new();
    for name in book.sheet_names() {
        let range = book.worksheet_range(&name).map_err(failed)?;
        tables.push(sheet(name, &range, header));
    }
    Ok(tables)
}
