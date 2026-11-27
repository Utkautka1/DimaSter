//! Вкладка: запрос, его результат и положение курсора в нём.

use dimaster_config::{ViewColumn, Viewer};
use dimaster_data::{DataError, Database, Table};

use crate::fuzzy;

pub(crate) struct Sheet {
    pub(crate) title: String,
    // запрос, которым получено показанное; уточняющие команды оборачивают его
    pub(crate) sql: String,
    original: String,
    pub(crate) table: Table,
    /// Видимые колонки; `column` и `left` — номера в этом списке.
    pub(crate) columns: Vec<ViewColumn>,
    /// Номера строк после нечёткого поиска, лучшие сначала.
    pub(crate) shown: Vec<usize>,
    pub(crate) row: usize,
    pub(crate) column: usize,
    // первые видимые строка и колонка
    pub(crate) top: usize,
    pub(crate) left: usize,
}

impl Sheet {
    pub(crate) fn open(db: &Database, name: &str) -> Result<Self, DataError> {
        let sql = format!("SELECT * FROM \"{}\"", name.replace('"', "\"\""));
        let table = db.query(&sql)?;
        let mut sheet = Self {
            title: name.to_owned(),
            original: sql.clone(),
            sql,
            table,
            columns: Vec::new(),
            shown: Vec::new(),
            row: 0,
            column: 0,
            top: 0,
            left: 0,
        };
        sheet.arrange(&Viewer::default());
        sheet.search("");
        Ok(sheet)
    }

    /// Новый результат; при ошибке вкладка остаётся как была.
    pub(crate) fn apply(&mut self, db: &Database, sql: String) -> Result<(), DataError> {
        self.table = db.query(&sql)?;
        self.sql = sql;
        (self.row, self.column, self.top, self.left) = (0, 0, 0, 0);
        self.columns.clear();
        self.search("");
        Ok(())
    }

    pub(crate) fn reset(&mut self, db: &Database) -> Result<(), DataError> {
        self.apply(db, self.original.clone())
    }

    /// Раскладывает колонки по правилам `view.lua`.
    pub(crate) fn arrange(&mut self, viewer: &Viewer) {
        self.columns = viewer.columns(&self.title, &self.table.columns);
        self.column = self.column.min(self.columns.len().saturating_sub(1));
    }

    /// Оставляет строки, похожие на запрос.
    pub(crate) fn search(&mut self, query: &str) {
        self.shown = fuzzy::filter(&self.table, query);
        self.row = 0;
        self.top = 0;
    }

    /// Сдвиг курсора с остановкой на краях.
    pub(crate) fn step(&mut self, rows: isize, columns: isize) {
        let last_row = self.shown.len().saturating_sub(1);
        let last_column = self.columns.len().saturating_sub(1);
        self.row = self.row.saturating_add_signed(rows).min(last_row);
        self.column = self.column.saturating_add_signed(columns).min(last_column);
    }

    /// Выбранная строка парами «колонка — значение».
    pub(crate) fn record(&self) -> Vec<(&str, String)> {
        let Some(row) = self.shown.get(self.row).map(|&i| &self.table.rows[i]) else {
            return Vec::new();
        };
        let names = self.table.columns.iter().map(String::as_str);
        names.zip(row.iter().map(|v| v.text())).collect()
    }
}
