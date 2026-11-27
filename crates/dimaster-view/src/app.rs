//! Состояние просмотрщика: вкладки и то, куда сейчас идут клавиши.

use dimaster_config::Viewer;
use dimaster_data::{DataError, Database};

use crate::keys::{self, Bindings};
use crate::plot::Plot;
use crate::sheet::Sheet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Table,
    /// Одна запись целиком; `true` — в виде JSON.
    Record(bool),
    /// Набирается команда после `:`.
    Command,
    /// Набирается нечёткий поиск после `/`.
    Search,
    /// Список вкладок.
    Tabs,
    /// График по текущей таблице.
    Plot,
}

pub struct App {
    pub(crate) db: Database,
    pub(crate) sheets: Vec<Sheet>,
    pub(crate) active: usize,
    pub(crate) mode: Mode,
    pub(crate) input: String,
    pub(crate) message: String,
    // колонки сжимаются, чтобы поместиться все
    pub(crate) fit: bool,
    // первая строка записи на экране
    pub(crate) scroll: usize,
    /// Строк таблицы на экране; задаётся отрисовкой.
    pub(crate) page: usize,
    /// Текст для буфера обмена, ждущий отправки терминалу.
    pub(crate) copied: Option<String>,
    pub(crate) done: bool,
    pub(crate) plot: Option<Plot>,
    pub(crate) viewer: Viewer,
    pub(crate) keys: Bindings,
}

impl App {
    /// По вкладке на каждую таблицу базы.
    pub fn new(db: Database) -> Result<Self, DataError> {
        let names = db.tables()?;
        let sheets: Result<Vec<_>, _> = names.iter().map(|n| Sheet::open(&db, n)).collect();
        let sheets = sheets?;
        if sheets.is_empty() {
            return Err(DataError::Empty);
        }
        Ok(Self {
            db,
            sheets,
            active: 0,
            mode: Mode::Table,
            input: String::new(),
            message: String::new(),
            fit: false,
            scroll: 0,
            page: 20,
            copied: None,
            done: false,
            plot: None,
            viewer: Viewer::default(),
            keys: keys::bindings(&[]),
        })
    }

    /// Применяет `view.lua`: колонки, вид и клавиши.
    #[must_use]
    pub fn with(mut self, viewer: Viewer) -> Self {
        self.fit = viewer.settings.fit;
        self.keys = keys::bindings(&viewer.settings.keys);
        self.viewer = viewer;
        for sheet in &mut self.sheets {
            sheet.arrange(&self.viewer);
        }
        self
    }

    /// Раскладывает колонки вкладок, получивших новый результат.
    pub(crate) fn arrange(&mut self) {
        for sheet in self.sheets.iter_mut().filter(|s| s.columns.is_empty()) {
            sheet.arrange(&self.viewer);
        }
    }

    pub(crate) fn sheet(&mut self) -> &mut Sheet {
        &mut self.sheets[self.active]
    }

    /// Закрывает вкладку; последняя закрывает программу.
    pub(crate) fn close(&mut self) {
        self.sheets.remove(self.active);
        self.done = self.sheets.is_empty();
        self.active = self.active.min(self.sheets.len().saturating_sub(1));
    }
}
