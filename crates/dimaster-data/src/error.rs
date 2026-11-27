//! Ошибки чтения и запросов. Тексты внешних библиотек передаются как есть:
//! приложение само добавляет к ним переведённое пояснение.

use std::fmt;

#[derive(Debug)]
pub enum DataError {
    Io(std::io::Error),
    /// Формат не определился по расширению.
    UnknownFormat,
    /// В файле не нашлось ни одной таблицы.
    Empty,
    /// Результат запроса не помещается в отведённую память.
    TooLarge,
    Parse(String),
    Sql(rusqlite::Error),
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::UnknownFormat => f.write_str("unknown format"),
            Self::Empty => f.write_str("no tables"),
            Self::TooLarge => f.write_str("result is too large"),
            Self::Parse(text) => f.write_str(text),
            Self::Sql(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DataError {}

impl From<std::io::Error> for DataError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<rusqlite::Error> for DataError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}

impl From<csv::Error> for DataError {
    fn from(e: csv::Error) -> Self {
        Self::Parse(e.to_string())
    }
}

impl From<serde_json::Error> for DataError {
    fn from(e: serde_json::Error) -> Self {
        Self::Parse(e.to_string())
    }
}
