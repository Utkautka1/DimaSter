//! Вкладка: результат, который читается порциями, и положение курсора в нём.

use dimaster_config::{ViewColumn, Viewer};
use dimaster_data::{DataError, Database, Rows, Value};

use crate::fuzzy;

// по стольким первым строкам подбираются ширины колонок
const SAMPLE: usize = 1000;

// откуда вкладка взялась: сюда возвращает сброс
enum Origin {
    Table(String),
    Query(String),
}

impl Origin {
    fn sql(&self) -> String {
        match self {
            Self::Table(name) => format!("SELECT * FROM \"{}\"", name.replace('"', "\"\"")),
            Self::Query(sql) => sql.clone(),
        }
    }

    fn open(&self, db: &Database) -> Result<Rows, DataError> {
        match self {
            Self::Table(name) => db.open_table(name),
            Self::Query(sql) => db.open(sql),
        }
    }
}

pub(crate) struct Sheet {
    pub(crate) title: String,
    // запрос, которым получено показанное; уточняющие команды оборачивают его
    pub(crate) sql: String,
    origin: Origin,
    pub(crate) data: Rows,
    pub(crate) sample: Vec<Vec<Value>>,
    /// Видимые колонки; `column` и `left` — номера в этом списке.
    pub(crate) columns: Vec<ViewColumn>,
    // номера строк после нечёткого поиска, лучшие сначала; `None` — все
    found: Option<Vec<usize>>,
    pub(crate) row: usize,
    pub(crate) column: usize,
    // первые видимые строка и колонка
    pub(crate) top: usize,
    pub(crate) left: usize,
}

impl Sheet {
    pub(crate) fn open(db: &Database, name: &str) -> Result<Self, DataError> {
        Self::from_origin(db, name.to_owned(), Origin::Table(name.to_owned()))
    }

    pub(crate) fn query(db: &Database, title: String, sql: String) -> Result<Self, DataError> {
        Self::from_origin(db, title, Origin::Query(sql))
    }

    fn from_origin(db: &Database, title: String, origin: Origin) -> Result<Self, DataError> {
        let data = origin.open(db)?;
        let mut sheet = Self {
            title,
            sql: origin.sql(),
            origin,
            sample: sample(db, &data)?,
            data,
            columns: Vec::new(),
            found: None,
            row: 0,
            column: 0,
            top: 0,
            left: 0,
        };
        sheet.arrange(&Viewer::default());
        Ok(sheet)
    }

    /// Новый результат; при ошибке вкладка остаётся как была.
    pub(crate) fn apply(&mut self, db: &Database, sql: String) -> Result<(), DataError> {
        let data = db.open(&sql)?;
        self.show(db, data, sql)
    }

    pub(crate) fn reset(&mut self, db: &Database) -> Result<(), DataError> {
        let data = self.origin.open(db)?;
        self.show(db, data, self.origin.sql())
    }

    fn show(&mut self, db: &Database, data: Rows, sql: String) -> Result<(), DataError> {
        self.sample = sample(db, &data)?;
        db.close(std::mem::replace(&mut self.data, data));
        self.sql = sql;
        (self.row, self.column, self.top, self.left) = (0, 0, 0, 0);
        self.columns.clear();
        self.found = None;
        Ok(())
    }

    /// Раскладывает колонки по правилам `view.lua`.
    pub(crate) fn arrange(&mut self, viewer: &Viewer) {
        self.columns = viewer.columns(&self.title, &self.data.columns);
        self.column = self.column.min(self.columns.len().saturating_sub(1));
    }

    /// Оставляет строки, похожие на запрос.
    pub(crate) fn search(&mut self, db: &Database, query: &str) -> Result<(), DataError> {
        self.found = fuzzy::filter(db, &self.data, query)?;
        self.row = 0;
        self.top = 0;
        Ok(())
    }

    /// Сколько строк показывается с учётом поиска.
    pub(crate) fn len(&self) -> usize {
        self.found.as_ref().map_or(self.data.len(), Vec::len)
    }

    /// Номера строк результата, прошедших поиск.
    pub(crate) fn found(&self) -> Option<&[usize]> {
        self.found.as_deref()
    }

    /// Строки экрана с `from` не дальше `count` штук.
    pub(crate) fn page(&self, db: &Database, from: usize, count: usize) -> Vec<Vec<Value>> {
        let end = from.saturating_add(count).min(self.len());
        let number = |at| self.found.as_ref().map_or(at, |f| f[at]);
        let indexes: Vec<usize> = (from..end).map(number).collect();
        db.fetch(&self.data, &indexes).unwrap_or_default()
    }

    /// Сдвиг курсора с остановкой на краях.
    pub(crate) fn step(&mut self, rows: isize, columns: isize) {
        let last_row = self.len().saturating_sub(1);
        let last_column = self.columns.len().saturating_sub(1);
        self.row = self.row.saturating_add_signed(rows).min(last_row);
        self.column = self.column.saturating_add_signed(columns).min(last_column);
    }

    /// Выбранная строка парами «колонка — значение».
    pub(crate) fn record(&self, db: &Database) -> Vec<(&str, String)> {
        let Some(row) = self.page(db, self.row, 1).pop() else {
            return Vec::new();
        };
        let names = self.data.columns.iter().map(String::as_str);
        names.zip(row.iter().map(Value::text)).collect()
    }
}

fn sample(db: &Database, data: &Rows) -> Result<Vec<Vec<Value>>, DataError> {
    let first: Vec<usize> = (0..data.len().min(SAMPLE)).collect();
    db.fetch(data, &first)
}
