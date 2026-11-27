//! SQLite в памяти со всеми загруженными таблицами.
//!
//! Базы с диска копируются в память и сразу отсоединяются, а `ATTACH` после
//! этого запрещён: ни запрос, ни чужой дамп не могут тронуть файлы.

use std::path::Path;

use rusqlite::limits::Limit;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, params_from_iter};

use crate::format::parse;
use crate::{DataError, Format, Options, Table, Value, is_leveldb, leveldb};

pub struct Database {
    connection: Connection,
}

fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn value(cell: ValueRef<'_>) -> Value {
    match cell {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(number) => Value::Int(number),
        ValueRef::Real(number) => Value::Real(number),
        ValueRef::Text(text) => Value::Text(String::from_utf8_lossy(text).into_owned()),
        ValueRef::Blob(bytes) => Value::Text(format!("<{} bytes>", bytes.len())),
    }
}

fn bound(cell: &Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sql;
    match cell {
        Value::Null => Sql::Null,
        Value::Int(number) => Sql::Integer(*number),
        Value::Real(number) => Sql::Real(*number),
        Value::Text(text) => Sql::Text(text.clone()),
    }
}

fn kind(table: &Table, column: usize) -> &'static str {
    let any = |test: fn(&Value) -> bool| table.rows.iter().any(|r| test(&r[column]));
    if any(|v| matches!(v, Value::Text(_))) {
        "TEXT"
    } else if any(|v| matches!(v, Value::Real(_))) {
        "REAL"
    } else {
        "INTEGER"
    }
}

impl Database {
    pub fn new() -> Result<Self, DataError> {
        let connection = Connection::open_in_memory()?;
        connection.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0)?;
        Ok(Self { connection })
    }

    /// Читает файл и возвращает имена добавленных таблиц.
    pub fn load(&mut self, path: &Path, options: &Options) -> Result<Vec<String>, DataError> {
        // база LevelDB — это папка, расширение тут ни при чём
        if is_leveldb(path) {
            let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
            let table = leveldb::read(path)?;
            return Ok(vec![self.add(stem.as_deref().unwrap_or("data"), &table)?]);
        }
        let format = options.format.or_else(|| Format::of(path));
        let stem = path
            .file_stem()
            .map_or_else(|| "data".into(), |s| s.to_string_lossy());
        match format.ok_or(DataError::UnknownFormat)? {
            Format::Sqlite => self.copy(path),
            Format::Sql => self.script(&std::fs::read_to_string(path)?),
            format => self.load_bytes(&stem, format, &std::fs::read(path)?, options),
        }
    }

    /// То же для уже прочитанного содержимого (например из конвейера).
    pub fn load_bytes(
        &mut self,
        name: &str,
        format: Format,
        bytes: &[u8],
        options: &Options,
    ) -> Result<Vec<String>, DataError> {
        if format == Format::Sql {
            return self.script(&String::from_utf8_lossy(bytes));
        }
        let tables = parse(format, bytes, options)?;
        let wanted = |t: &Table| match t.name.is_empty() {
            true => name.to_owned(),
            false => t.name.clone(),
        };
        let added: Result<Vec<_>, _> = tables.iter().map(|t| self.add(&wanted(t), t)).collect();
        Some(added?)
            .filter(|a| !a.is_empty())
            .ok_or(DataError::Empty)
    }

    /// Все таблицы и представления в порядке создания.
    pub fn tables(&self) -> Result<Vec<String>, DataError> {
        let sql = "SELECT name FROM sqlite_schema WHERE type IN ('table', 'view') \
                   AND name NOT LIKE 'sqlite_%' ORDER BY rowid";
        let found = self.query(sql)?;
        Ok(found.rows.iter().map(|r| r[0].text()).collect())
    }

    // свободное имя: `data`, `data_2`, …
    fn free_name(&self, wanted: &str) -> Result<String, DataError> {
        let taken = self.tables()?;
        let clashes = |name: &str| taken.iter().any(|t| t.eq_ignore_ascii_case(name));
        let mut name = wanted.to_owned();
        let mut number = 2;
        while clashes(&name) {
            name = format!("{wanted}_{number}");
            number += 1;
        }
        Ok(name)
    }

    fn add(&mut self, wanted: &str, table: &Table) -> Result<String, DataError> {
        let name = self.free_name(wanted)?;
        let columns: Vec<String> = (0..table.columns.len())
            .map(|i| format!("{} {}", quote(&table.columns[i]), kind(table, i)))
            .collect();
        let create = format!("CREATE TABLE {} ({})", quote(&name), columns.join(", "));
        self.connection.execute(&create, [])?;
        self.insert(&name, table)?;
        Ok(name)
    }

    fn insert(&mut self, name: &str, table: &Table) -> Result<(), DataError> {
        let marks = vec!["?"; table.columns.len()].join(", ");
        let sql = format!("INSERT INTO {} VALUES ({marks})", quote(name));
        // одна транзакция на всю таблицу: иначе вставка в сотни раз медленнее
        let transaction = self.connection.transaction()?;
        {
            let mut statement = transaction.prepare(&sql)?;
            for row in &table.rows {
                statement.execute(params_from_iter(row.iter().map(bound)))?;
            }
        }
        Ok(transaction.commit()?)
    }

    fn added_since(&self, before: &[String]) -> Result<Vec<String>, DataError> {
        let mut added = self.tables()?;
        added.retain(|name| !before.contains(name));
        Some(added)
            .filter(|a| !a.is_empty())
            .ok_or(DataError::Empty)
    }

    fn script(&mut self, sql: &str) -> Result<Vec<String>, DataError> {
        let before = self.tables()?;
        self.connection.execute_batch(sql)?;
        self.added_since(&before)
    }

    // копия таблиц базы с диска; сама база только читается
    fn copy(&mut self, path: &Path) -> Result<Vec<String>, DataError> {
        // отсутствующий файл ATTACH молча создал бы пустым
        std::fs::metadata(path)?;
        let before = self.tables()?;
        self.connection.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 1)?;
        let file = path.to_string_lossy();
        self.connection
            .execute("ATTACH ?1 AS source", [file.as_ref()])?;
        let result = self.copy_attached();
        self.connection.execute_batch("DETACH source")?;
        self.connection.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0)?;
        result?;
        self.added_since(&before)
    }

    fn copy_attached(&self) -> Result<(), DataError> {
        let sql = "SELECT name FROM source.sqlite_schema WHERE type IN ('table', 'view') \
                   AND name NOT LIKE 'sqlite_%'";
        for row in self.query(sql)?.rows {
            let original = row[0].text();
            let name = self.free_name(&original)?;
            let create = format!(
                "CREATE TABLE main.{} AS SELECT * FROM source.{}",
                quote(&name),
                quote(&original)
            );
            self.connection.execute(&create, [])?;
        }
        Ok(())
    }

    /// Результат первого оператора запроса.
    pub fn query(&self, sql: &str) -> Result<Table, DataError> {
        let mut statement = self.connection.prepare(sql)?;
        let names = statement.column_names();
        let columns: Vec<String> = names.into_iter().map(str::to_owned).collect();
        let width = columns.len();
        let mut found = statement.query([])?;
        let mut rows = Vec::new();
        while let Some(row) = found.next()? {
            let cells: Result<Vec<_>, _> = (0..width).map(|i| row.get_ref(i).map(value)).collect();
            rows.push(cells?);
        }
        Ok(Table {
            name: String::new(),
            columns,
            rows,
        })
    }

    /// Таблица целиком.
    pub fn table(&self, name: &str) -> Result<Table, DataError> {
        let mut table = self.query(&format!("SELECT * FROM {}", quote(name)))?;
        table.name = name.to_owned();
        Ok(table)
    }
}
