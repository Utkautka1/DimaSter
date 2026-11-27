//! Определение формата и разбор текста в таблицы.

use std::path::Path;

use crate::{DataError, Table, delimited, fwf, json, logfmt, markup};

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

/// Таблицы из текста; базы и дампы SQL сюда не попадают.
pub(crate) fn parse(
    format: Format,
    text: &str,
    options: &Options,
) -> Result<Vec<Table>, DataError> {
    let text = text.trim_start_matches('\u{feff}');
    let header = !options.no_header;
    let comma = if format == Format::Tsv { b'\t' } else { b',' };
    let tables = match format {
        Format::Csv | Format::Tsv => {
            vec![delimited::parse(
                text,
                options.separator.unwrap_or(comma),
                header,
            )?]
        }
        Format::Json => vec![json::parse(text)?],
        Format::Jsonl => vec![json::parse_lines(text)?],
        Format::Fwf => vec![fwf::parse(text, header)],
        Format::Logfmt => vec![logfmt::parse(text)],
        Format::Html => markup::html(text),
        Format::Markdown => markup::markdown(text),
        Format::Sqlite | Format::Sql => return Err(DataError::UnknownFormat),
    };
    Ok(tables
        .into_iter()
        .filter(|t| !t.columns.is_empty())
        .collect())
}
