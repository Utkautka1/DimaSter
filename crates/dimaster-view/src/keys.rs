//! Клавиши в духе vim, как в tabiew.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Mode};
use crate::command;

// команды, открывающиеся уже с префиксом
/// Что умеют клавиши таблицы; имена — те, что пишутся в `view.lua`.
const DEFAULTS: [(&str, &str); 36] = [
    ("j", "down"),
    ("down", "down"),
    ("k", "up"),
    ("up", "up"),
    ("h", "left"),
    ("b", "left"),
    ("left", "left"),
    ("l", "right"),
    ("w", "right"),
    ("right", "right"),
    ("ctrl+d", "half_down"),
    ("ctrl+u", "half_up"),
    ("ctrl+f", "page_down"),
    ("pagedown", "page_down"),
    ("ctrl+b", "page_up"),
    ("pageup", "page_up"),
    ("g", "top"),
    ("home", "top"),
    ("G", "bottom"),
    ("end", "bottom"),
    ("ctrl+r", "reset"),
    ("enter", "record"),
    ("c", "copy"),
    ("e", "fit"),
    ("t", "tabs"),
    (":", "command"),
    ("/", "search"),
    ("s", "select"),
    ("o", "order"),
    ("f", "filter"),
    ("q", "close"),
    ("Q", "quit"),
    ("tab", "next_tab"),
    ("backtab", "previous_tab"),
    ("0", "first_column"),
    ("$", "last_column"),
];

pub(crate) type Bindings = HashMap<String, String>;

/// Клавиши по умолчанию с изменениями пользователя.
pub(crate) fn bindings(changes: &[(String, Option<String>)]) -> Bindings {
    let own = |(key, action): &(&str, &str)| ((*key).to_owned(), (*action).to_owned());
    let mut found: Bindings = DEFAULTS.iter().map(own).collect();
    for (key, action) in changes {
        match action {
            Some(action) => found.insert(key.clone(), action.clone()),
            None => found.remove(key),
        };
    }
    found
}

// запись нажатия так, как её пишут в `view.lua`
fn name(key: KeyEvent) -> String {
    let base = match key.code {
        KeyCode::Char(' ') => "space".to_owned(),
        KeyCode::Char(ch) => ch.to_string(),
        KeyCode::F(number) => format!("f{number}"),
        other => format!("{other:?}").to_ascii_lowercase(),
    };
    match key.modifiers {
        m if m.contains(KeyModifiers::CONTROL) => format!("ctrl+{base}"),
        m if m.contains(KeyModifiers::ALT) => format!("alt+{base}"),
        _ => base,
    }
}

fn prompt(app: &mut App, mode: Mode, start: &str) {
    app.mode = mode;
    app.input = start.to_owned();
}

// перемещение по таблице; `false` — действие не о нём
fn movement(app: &mut App, action: &str) -> bool {
    let page = app.page.max(1) as isize;
    let (rows, columns) = match action {
        "down" => (1, 0),
        "up" => (-1, 0),
        "left" => (0, -1),
        "right" => (0, 1),
        "half_down" => (page / 2, 0),
        "half_up" => (-page / 2, 0),
        "page_down" => (page, 0),
        "page_up" => (-page, 0),
        "top" => (isize::MIN, 0),
        "bottom" => (isize::MAX, 0),
        "first_column" => (0, isize::MIN),
        "last_column" => (0, isize::MAX),
        _ => return false,
    };
    app.sheet().step(rows, columns);
    true
}

// выбранная запись в буфер обмена, значения через табуляцию
fn copy(app: &mut App) {
    let values: Vec<String> = app.sheet().record().into_iter().map(|p| p.1).collect();
    app.copied = Some(values.join("\t"));
    app.message = dimaster_i18n::tr("Строка скопирована");
}

fn reset(app: &mut App) {
    let sheet = &mut app.sheets[app.active];
    if let Err(e) = sheet.reset(&app.db) {
        app.message = e.to_string();
    }
}

fn turn(app: &mut App, step: usize) {
    app.active = (app.active + step) % app.sheets.len();
}

fn act(app: &mut App, action: &str) {
    if movement(app, action) {
        return;
    }
    match action {
        "reset" => reset(app),
        "record" => (app.mode, app.scroll) = (Mode::Record(false), 0),
        "copy" => copy(app),
        "fit" => app.fit = !app.fit,
        "tabs" => app.mode = Mode::Tabs,
        "next_tab" => turn(app, 1),
        "previous_tab" => turn(app, app.sheets.len() - 1),
        "command" => prompt(app, Mode::Command, ""),
        "search" => prompt(app, Mode::Search, ""),
        "select" => prompt(app, Mode::Command, "s "),
        "order" => prompt(app, Mode::Command, "o "),
        "filter" => prompt(app, Mode::Command, "f "),
        "close" => app.close(),
        "quit" => app.done = true,
        _ => {}
    }
}

fn record(app: &mut App, code: KeyCode, as_json: bool) {
    match code {
        KeyCode::Char('f') => app.mode = Mode::Record(!as_json),
        KeyCode::Char('j') | KeyCode::Down => app.scroll += 1,
        KeyCode::Char('k') | KeyCode::Up => app.scroll = app.scroll.saturating_sub(1),
        KeyCode::Char('c') => copy(app),
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => app.mode = Mode::Table,
        _ => {}
    }
}

fn tabs(app: &mut App, code: KeyCode) {
    let count = app.sheets.len();
    match code {
        KeyCode::Char('j') | KeyCode::Down => app.active = (app.active + 1) % count,
        KeyCode::Char('k') | KeyCode::Up => app.active = (app.active + count - 1) % count,
        KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q' | 't') => app.mode = Mode::Table,
        _ => {}
    }
}

fn submit(app: &mut App) {
    let line = std::mem::take(&mut app.input);
    let command = app.mode == Mode::Command;
    // команда может сама сменить режим, например открыть график
    app.mode = Mode::Table;
    if command && !line.trim().is_empty() {
        app.message = command::run(app, &line).unwrap_or_else(|e| e);
    }
}

// набор команды или поиска; поиск применяется на каждую букву
fn typing(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter => return submit(app),
        KeyCode::Esc => {
            if app.mode == Mode::Search {
                app.sheet().search("");
            }
            app.mode = Mode::Table;
            return;
        }
        KeyCode::Backspace => drop(app.input.pop()),
        KeyCode::Char(ch) => app.input.push(ch),
        _ => return,
    }
    if app.mode == Mode::Search {
        let query = app.input.clone();
        app.sheet().search(&query);
    }
}

impl App {
    /// Команда извне, как набранная после `:`; `quit` закрывает программу.
    pub(crate) fn remote(&mut self, line: &str) -> String {
        self.mode = Mode::Table;
        if line == "quit" {
            self.done = true;
            return "ok".to_owned();
        }
        let answer = command::run(self, line);
        self.arrange();
        match answer {
            Ok(message) => {
                self.message = message;
                "ok".to_owned()
            }
            Err(e) => {
                self.message.clone_from(&e);
                format!("error: {e}")
            }
        }
    }

    /// Обрабатывает нажатие клавиши.
    pub(crate) fn press(&mut self, key: KeyEvent) {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        self.message.clear();
        match self.mode {
            _ if control && key.code == KeyCode::Char('c') => self.done = true,
            Mode::Table => {
                if let Some(action) = self.keys.get(&name(key)).cloned() {
                    act(self, &action);
                }
            }
            Mode::Record(json) => record(self, key.code, json),
            Mode::Tabs => tabs(self, key.code),
            Mode::Plot => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
                    self.mode = Mode::Table;
                }
            }
            Mode::Command | Mode::Search => typing(self, key.code),
        }
        if !self.done {
            self.arrange();
        }
    }
}

#[cfg(test)]
mod tests {
    use dimaster_data::{Database, Format, Options};

    use super::*;

    fn app() -> App {
        let mut db = Database::new().expect("база");
        let text = "name,age\nann,31\nbob,25\n";
        db.load_bytes("people", Format::Csv, text.as_bytes(), &Options::default())
            .expect("csv");
        App::new(db).expect("приложение")
    }

    fn type_keys(app: &mut App, text: &str) {
        for ch in text.chars() {
            let code = match ch {
                '\n' => KeyCode::Enter,
                other => KeyCode::Char(other),
            };
            app.press(KeyEvent::new(code, KeyModifiers::NONE));
        }
    }

    #[test]
    fn commands_refine_the_result() {
        let mut app = app();
        type_keys(&mut app, "o age\n");
        assert_eq!(app.sheet().record()[0].1, "bob");
        type_keys(&mut app, ":q SELECT name FROM _ WHERE age > 30\n");
        assert_eq!(app.sheet().record(), [("name", "ann".to_owned())]);
        type_keys(&mut app, ":f nonsense\n");
        assert!(app.message.contains("nonsense"));
        assert_eq!(app.sheet().table.rows.len(), 1);
    }

    #[test]
    fn lua_rebinds_keys() {
        let viewer = dimaster_config::Viewer::parse(
            "return { keys = { x = 'quit', q = false }, columns = { hidden = { 'name' } } }",
        );
        let mut app = app().with(viewer.expect("view.lua"));
        assert_eq!(app.sheet().columns.len(), 1);
        type_keys(&mut app, "q");
        assert!(!app.done);
        type_keys(&mut app, "x");
        assert!(app.done);
    }

    #[test]
    fn plots_and_remote_commands() {
        let mut app = app();
        type_keys(&mut app, ":hist age\n");
        assert_eq!(app.mode, Mode::Plot);
        type_keys(&mut app, "q");
        assert_eq!((app.mode, app.done), (Mode::Table, false));
        assert_eq!(app.remote("o age"), "ok");
        assert_eq!(app.sheet().record()[0].1, "bob");
        assert!(app.remote("nope").starts_with("error"));
        app.remote("quit");
        assert!(app.done);
    }

    #[test]
    fn search_copy_and_close() {
        let mut app = app();
        type_keys(&mut app, "/bo\nc");
        assert_eq!(app.copied.as_deref(), Some("bob\t25"));
        type_keys(&mut app, "q");
        assert!(app.done);
    }
}
