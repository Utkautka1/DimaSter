//! Результат запроса, читаемый порциями.
//!
//! Копии строк в памяти нет: загруженная таблица читается прямо из SQLite по
//! номеру строки. Результат запроса один раз складывается во временную таблицу,
//! чтобы листание не выполняло запрос заново.

use rusqlite::{ErrorCode, params};

use crate::database::{quote, value};
use crate::limit::Watchdog;
use crate::{DataError, Database, Value};

// имена, под которыми SQLite отдаёт номер строки, если их не заняли колонки
const KEYS: [&str; 3] = ["_rowid_", "rowid", "oid"];

#[derive(Debug)]
enum Source {
    // номера строк идут подряд с единицы
    Stored {
        table: String,
        key: &'static str,
        temporary: bool,
    },
    // PRAGMA и прочее, что нельзя сложить в таблицу: такие ответы малы
    Memory(Vec<Vec<Value>>),
}

/// Колонки и строки результата.
#[derive(Debug)]
pub struct Rows {
    pub columns: Vec<String>,
    len: usize,
    source: Source,
}

impl Rows {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

fn is_select(sql: &str) -> bool {
    let word = sql.split_whitespace().next().unwrap_or_default();
    word.eq_ignore_ascii_case("select") || word.eq_ignore_ascii_case("with")
}

fn too_large(error: rusqlite::Error) -> DataError {
    match error.sqlite_error_code() {
        Some(ErrorCode::DiskFull) => DataError::TooLarge,
        _ => error.into(),
    }
}

impl Database {
    /// Таблица целиком, без копирования.
    pub fn open_table(&self, name: &str) -> Result<Rows, DataError> {
        let table = format!("main.{}", quote(name));
        let columns = self.names(&format!("SELECT * FROM {table}"))?;
        match self.stored(&table, columns, false)? {
            Some(rows) => Ok(rows),
            // представление или дыры в номерах: нужна своя копия
            None => self.open(&format!("SELECT * FROM {table}")),
        }
    }

    /// Результат запроса.
    pub fn open(&self, sql: &str) -> Result<Rows, DataError> {
        let number = self.next_view();
        let table = format!("temp.dimaster_view_{number}");
        let create = format!("CREATE TABLE {table} AS {sql}");
        if !is_select(sql) || self.connection.prepare(&create).is_err() {
            return self.in_memory(sql);
        }
        // имена берутся из запроса: таблица переименовала бы повторяющиеся
        let columns = self.names(sql)?;
        let watchdog = Watchdog::start(&self.connection, self.limits.time);
        self.connection.execute(&create, []).map_err(too_large)?;
        drop(watchdog);
        let stored = self.stored(&table, columns, true)?;
        stored.ok_or(DataError::Empty)
    }

    fn in_memory(&self, sql: &str) -> Result<Rows, DataError> {
        let table = self.query(sql)?;
        Ok(Rows {
            columns: table.columns,
            len: table.rows.len(),
            source: Source::Memory(table.rows),
        })
    }

    fn names(&self, sql: &str) -> Result<Vec<String>, DataError> {
        let statement = self.connection.prepare(sql)?;
        let names = statement.column_names();
        Ok(names.into_iter().map(str::to_owned).collect())
    }

    // `None`, если строки нельзя адресовать по номеру
    fn stored(
        &self,
        table: &str,
        columns: Vec<String>,
        temporary: bool,
    ) -> Result<Option<Rows>, DataError> {
        let taken = self.names(&format!("SELECT * FROM {table}"))?;
        let free = |key: &&str| !taken.iter().any(|c| c.eq_ignore_ascii_case(key));
        let Some(key) = KEYS.into_iter().find(free) else {
            return Ok(None);
        };
        let Some(len) = self.dense(table, key) else {
            return Ok(None);
        };
        let source = Source::Stored {
            table: table.to_owned(),
            key,
            temporary,
        };
        Ok(Some(Rows {
            columns,
            len,
            source,
        }))
    }

    // число строк, если их номера — ровно 1..=n
    fn dense(&self, table: &str, key: &str) -> Option<usize> {
        let sql = format!("SELECT count(*), min({key}), max({key}) FROM {table}");
        let found = self.connection.query_row(&sql, [], |row| {
            Ok((row.get::<_, i64>(0)?, row.get(1)?, row.get(2)?))
        });
        let (count, low, high): (i64, Option<i64>, Option<i64>) = found.ok()?;
        let dense = count == 0 || (low == Some(1) && high == Some(count));
        usize::try_from(count).ok().filter(|_| dense)
    }

    /// Строки с этими номерами (с нуля); пропавшая строка приходит пустой.
    pub fn fetch(&self, rows: &Rows, indexes: &[usize]) -> Result<Vec<Vec<Value>>, DataError> {
        let blank = || vec![Value::Null; rows.columns.len()];
        let (table, key) = match &rows.source {
            Source::Memory(all) => {
                let pick = |&i: &usize| all.get(i).cloned().unwrap_or_else(blank);
                return Ok(indexes.iter().map(pick).collect());
            }
            Source::Stored { table, key, .. } => (table, key),
        };
        let sql = format!("SELECT * FROM {table} WHERE {key} = ?1");
        let mut statement = self.connection.prepare_cached(&sql)?;
        let width = rows.columns.len();
        let mut one = |index: usize| -> Result<Vec<Value>, DataError> {
            let mut found = statement.query(params![index as i64 + 1])?;
            let Some(row) = found.next()? else {
                return Ok(blank());
            };
            let cells = (0..width).map(|i| row.get_ref(i).map(value));
            Ok(cells.collect::<Result<_, _>>()?)
        };
        indexes.iter().map(|&i| one(i)).collect()
    }

    /// Обходит все строки по порядку, держа в памяти одну.
    pub fn scan(
        &self,
        rows: &Rows,
        mut visit: impl FnMut(&[Value]) -> Result<(), DataError>,
    ) -> Result<(), DataError> {
        let (table, key) = match &rows.source {
            Source::Memory(all) => return all.iter().try_for_each(|r| visit(r)),
            Source::Stored { table, key, .. } => (table, key),
        };
        let sql = format!("SELECT * FROM {table} ORDER BY {key}");
        let mut statement = self.connection.prepare(&sql)?;
        let mut found = statement.query([])?;
        let mut cells = Vec::with_capacity(rows.columns.len());
        while let Some(row) = found.next()? {
            cells.clear();
            for i in 0..rows.columns.len() {
                cells.push(value(row.get_ref(i)?));
            }
            visit(&cells)?;
        }
        Ok(())
    }

    /// Освобождает временную таблицу результата.
    pub fn close(&self, rows: Rows) {
        if let Source::Stored {
            table,
            temporary: true,
            ..
        } = rows.source
        {
            // не вышло — таблица исчезнет вместе с программой
            let _ = self
                .connection
                .execute_batch(&format!("DROP TABLE {table}"));
        }
    }
}
