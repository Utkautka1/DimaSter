//! Табличные файлы для `dimaster view`.
//!
//! Любой формат читается в [`Table`], а таблицы складываются в SQLite в
//! памяти. Так SQL и `JOIN` между разными файлами достаются без своего
//! движка запросов, а исходные файлы никогда не изменяются.

mod columnar;
mod database;
mod delimited;
mod error;
mod excel;
mod format;
mod fwf;
mod json;
mod leveldb;
mod limit;
mod logfmt;
mod markup;
mod rows;
mod table;

pub use database::Database;
pub use error::DataError;
pub use format::{Format, Options};
pub use leveldb::is_leveldb;
pub use limit::Limits;
pub use rows::Rows;
pub use table::{Table, Value};
