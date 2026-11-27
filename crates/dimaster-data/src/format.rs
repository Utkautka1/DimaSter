//! Определение формата и разбор текста в таблицы.

use std::path::Path;

use crate::{DataError, Table, Value, columnar, delimited, excel, fwf, json, logfmt, markup};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Csv,
    Tsv,
    Json,
    Jsonl,
    Fwf,
    Logfmt,
    Html,
    Markdown,
    Sqlite,
    Parquet,
    /// Arrow IPC: файл или поток.
    Arrow,
    /// xlsx, xls, xlsm, xlsb и ods; по таблице на лист.
    Excel,
    /// Обычный журнал: каждая строка — запись в колонке `line`.
    Log,
    /// Дамп с командами SQL.
    Sql,
}

impl Format {
    /// По имени из `-f` или по расширению.
    pub fn named(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "csv" => Self::Csv,
            "tsv" => Self::Tsv,
            "json" => Self::Json,
            "jsonl" | "ndjson" => Self::Jsonl,
            "fwf" => Self::Fwf,
            "logfmt" => Self::Logfmt,
            "html" | "htm" => Self::Html,
            "md" | "markdown" => Self::Markdown,
            "db" | "sqlite" | "sqlite3" => Self::Sqlite,
            "sql" => Self::Sql,
            "log" | "txt" => Self::Log,
            "parquet" | "pqt" => Self::Parquet,
            "arrow" | "feather" | "ipc" => Self::Arrow,
            "excel" | "xlsx" | "xls" | "xlsm" | "xlsb" | "ods" => Self::Excel,
            _ => return None,
        })
    }

    pub fn of(path: &Path) -> Option<Self> {
        Self::named(path.extension()?.to_str()?)
    }
}

/// Уточнения из командной строки.
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub format: Option<Format>,
    /// Разделитель CSV; без него `,` или табуляция по формату.
    pub separator: Option<u8>,
    pub no_header: bool,
}

// строки как есть: без угадывания чисел и разбора кавычек
fn lines(text: &str) -> Table {
    let row = |line: &str| vec![Value::Text(line.to_owned())];
    Table::new(vec!["line".to_owned()], text.lines().map(row).collect())
}

// лист или пустое имя — тогда таблицу назовут по файлу
fn text_tables(format: Format, text: &str, options: &Options) -> Result<Vec<Table>, DataError> {
    let text = text.trim_start_matches('\u{feff}');
    let header = !options.no_header;
    let comma = if format == Format::Tsv { b'\t' } else { b',' };
    let separator = options.separator.unwrap_or(comma);
    Ok(match format {
        Format::Csv | Format::Tsv => vec![delimited::parse(text, separator, header)?],
        Format::Json => vec![json::parse(text)?],
        Format::Jsonl => vec![json::parse_lines(text)?],
        Format::Fwf => vec![fwf::parse(text, header)],
        Format::Logfmt => vec![logfmt::parse(text)],
        Format::Html => markup::html(text),
        Format::Markdown => markup::markdown(text),
        Format::Log => vec![lines(text)],
        _ => return Err(DataError::UnknownFormat),
    })
}

/// Таблицы из содержимого файла; базы и дампы SQL сюда не попадают.
pub(crate) fn parse(
    format: Format,
    bytes: &[u8],
    options: &Options,
) -> Result<Vec<Table>, DataError> {
    let tables = match format {
        Format::Parquet => vec![columnar::parquet(bytes)?],
        Format::Arrow => vec![columnar::arrow(bytes)?],
        Format::Excel => excel::parse(bytes, !options.no_header)?,
        text => text_tables(text, &String::from_utf8_lossy(bytes), options)?,
    };
    Ok(tables
        .into_iter()
        .filter(|t| !t.columns.is_empty())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_keeps_every_line() {
        let tables = parse(Format::Log, b"a \"b, c\n\n007\n", &Options::default()).expect("лог");
        let rows: Vec<String> = tables[0].rows.iter().map(|r| r[0].text()).collect();
        assert_eq!(rows, ["a \"b, c", "", "007"]);
    }
}
