//! Командная строка `:` — `q` запрос, `f` фильтр, `o` порядок, `s` колонки,
//! `hist` и `scatter`, `schema`, `import` и `export`.

use std::path::Path;

use dimaster_data::{Database, Options, Table};
use dimaster_i18n::{t, tr};

use crate::app::{App, Mode};
use crate::plot;
use crate::sheet::Sheet;

// `_` в запросе — то, что сейчас на экране
fn query(current: &str, sql: &str) -> String {
    let sql = sql.trim().trim_end_matches(';');
    let selects = sql
        .get(..6)
        .is_some_and(|w| w.eq_ignore_ascii_case("select"));
    match selects {
        true => format!("WITH _ AS ({current}) {sql}"),
        false => sql.to_owned(),
    }
}

// SQL для команды, уточняющей текущий результат
fn refine(current: &str, word: &str, rest: &str) -> Option<String> {
    Some(match word {
        "q" | "query" => query(current, rest),
        "f" | "filter" => format!("SELECT * FROM ({current}) WHERE {rest}"),
        "o" | "order" => format!("SELECT * FROM ({current}) ORDER BY {rest}"),
        "s" | "select" => format!("SELECT {rest} FROM ({current})"),
        _ => return None,
    })
}

fn csv_field(text: &str) -> String {
    match text.contains([',', '"', '\n', '\r']) {
        true => format!("\"{}\"", text.replace('"', "\"\"")),
        false => text.to_owned(),
    }
}

/// Таблица в CSV с заголовком.
pub(crate) fn to_csv(table: &Table) -> String {
    let line = |cells: Vec<String>| cells.join(",") + "\n";
    let mut out = line(table.columns.iter().map(|c| csv_field(c)).collect());
    for row in &table.rows {
        out += &line(row.iter().map(|v| csv_field(&v.text())).collect());
    }
    out
}

fn export(app: &mut App, path: &str) -> Result<String, String> {
    let table = &app.sheet().table;
    // чужой файл не перезаписываем молча
    let file = std::fs::File::options()
        .write(true)
        .create_new(true)
        .open(path);
    file.and_then(|mut f| std::io::Write::write_all(&mut f, to_csv(table).as_bytes()))
        .map_err(|e| e.to_string())?;
    Ok(t!("Сохранено: {path}", path = path))
}

fn import(app: &mut App, path: &str) -> Result<String, String> {
    let added = app.db.load(Path::new(path), &Options::default());
    let added = added.map_err(|e| e.to_string())?;
    for name in &added {
        let sheet = Sheet::open(&app.db, name).map_err(|e| e.to_string())?;
        app.sheets.push(sheet);
    }
    app.active = app.sheets.len() - 1;
    Ok(String::new())
}

// список колонок всех таблиц отдельной вкладкой
fn schema(db: &Database) -> Result<Sheet, String> {
    let sql = "SELECT t.name AS \"table\", c.name AS \"column\", c.type AS type \
               FROM sqlite_schema t, pragma_table_info(t.name) c WHERE t.type = 'table'";
    let mut sheet = Sheet::open(db, "sqlite_schema").map_err(|e| e.to_string())?;
    sheet.apply(db, sql.to_owned()).map_err(|e| e.to_string())?;
    sheet.title = tr("Схема");
    Ok(sheet)
}

/// Выполняет строку команды; `Ok` — сообщение для строки состояния.
pub(crate) fn run(app: &mut App, line: &str) -> Result<String, String> {
    let line = line.trim();
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    match word {
        "export" if !rest.is_empty() => return export(app, rest.trim()),
        "import" if !rest.is_empty() => return import(app, rest.trim()),
        "schema" => {
            app.sheets.push(schema(&app.db)?);
            app.active = app.sheets.len() - 1;
            return Ok(String::new());
        }
        _ => {}
    }
    if let Some(parsed) = plot::parse(&app.sheet().table, word, rest) {
        (app.plot, app.mode) = (Some(parsed?), Mode::Plot);
        return Ok(String::new());
    }
    let current = app.sheet().sql.clone();
    let sql = refine(&current, word, rest).filter(|_| !rest.is_empty());
    let sql = sql.ok_or_else(|| t!("Неизвестная команда: {command}", command = line))?;
    let sheet = &mut app.sheets[app.active];
    sheet.apply(&app.db, sql).map_err(|e| e.to_string())?;
    Ok(String::new())
}
