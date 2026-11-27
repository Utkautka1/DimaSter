//! Клавиши экрана контейнеров.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use dimaster_view::key_name;

use crate::app::{App, Mode};

const DEFAULTS: &[(&str, &str)] = &[
    ("j", "down"),
    ("down", "down"),
    ("k", "up"),
    ("up", "up"),
    ("ctrl+d", "page_down"),
    ("pagedown", "page_down"),
    ("ctrl+u", "page_up"),
    ("pageup", "page_up"),
    ("g", "top"),
    ("home", "top"),
    ("G", "bottom"),
    ("end", "bottom"),
    ("tab", "next_tab"),
    ("backtab", "previous_tab"),
    ("h", "next_host"),
    ("H", "previous_host"),
    ("s", "toggle"),
    ("r", "restart"),
    ("p", "pause"),
    ("d", "remove"),
    ("D", "prune"),
    ("u", "up"),
    ("x", "down_project"),
    ("l", "logs"),
    ("enter", "shell"),
    ("i", "inspect"),
    ("/", "filter"),
    ("R", "refresh"),
    ("q", "quit"),
];

// своё назначение из `docker.lua` важнее; `false` в нём снимает клавишу
fn action(app: &App, key: &str) -> Option<String> {
    let own = app.config.view.settings.keys.iter().find(|k| k.0 == key);
    match own {
        Some((_, action)) => action.clone(),
        None => DEFAULTS.iter().find(|d| d.0 == key).map(|d| d.1.to_owned()),
    }
}

fn step(app: &mut App, delta: isize) {
    let last = app.visible().len().saturating_sub(1);
    app.selected = app.selected.saturating_add_signed(delta).min(last);
}

fn act(app: &mut App, name: &str) {
    let page = app.page.max(1) as isize;
    match name {
        "down" => step(app, 1),
        "up" => step(app, -1),
        "page_down" => step(app, page),
        "page_up" => step(app, -page),
        "top" => step(app, isize::MIN),
        "bottom" => step(app, isize::MAX),
        "next_tab" => app.turn_tab(1),
        "previous_tab" => app.turn_tab(crate::kind::KINDS.len() - 1),
        "next_host" => app.turn_host(1),
        "previous_host" => app.turn_host(app.host_count() - 1),
        "filter" => app.mode = Mode::Filter,
        "refresh" => app.refresh(),
        "quit" => app.done = true,
        "down_project" => app.act("down"),
        other => app.act(other),
    }
}

fn filter(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter => app.mode = Mode::List,
        KeyCode::Esc => {
            app.filter.clear();
            app.mode = Mode::List;
        }
        KeyCode::Backspace => drop(app.filter.pop()),
        KeyCode::Char(ch) => app.filter.push(ch),
        _ => return,
    }
    (app.selected, app.top) = (0, 0);
}

// прокрутка вывода; `back` считается от конца
fn scrolled(value: usize, code: KeyCode, page: usize) -> usize {
    match code {
        KeyCode::Char('j') | KeyCode::Down => value.saturating_add(1),
        KeyCode::Char('k') | KeyCode::Up => value.saturating_sub(1),
        KeyCode::PageDown => value.saturating_add(page),
        KeyCode::PageUp => value.saturating_sub(page),
        KeyCode::Char('g') | KeyCode::Home => 0,
        KeyCode::Char('G') | KeyCode::End => usize::MAX,
        _ => value,
    }
}

// в логах вниз — к свежим строкам, то есть меньше отступ от конца
fn backwards(code: KeyCode) -> KeyCode {
    match code {
        KeyCode::Char('j') | KeyCode::Down => KeyCode::Up,
        KeyCode::Char('k') | KeyCode::Up => KeyCode::Down,
        KeyCode::PageDown => KeyCode::PageUp,
        KeyCode::PageUp => KeyCode::PageDown,
        KeyCode::Char('g') | KeyCode::Home => KeyCode::End,
        KeyCode::Char('G') | KeyCode::End => KeyCode::Home,
        other => other,
    }
}

fn output(app: &mut App, code: KeyCode) {
    if matches!(code, KeyCode::Esc | KeyCode::Char('q')) {
        app.mode = Mode::List;
        return;
    }
    let page = app.page;
    match &mut app.mode {
        Mode::Text { lines, scroll } => {
            *scroll = scrolled(*scroll, code, page).min(lines.len().saturating_sub(1));
        }
        Mode::Logs { tail, back } => {
            let most = tail.lines.len().saturating_sub(page);
            *back = scrolled(*back, backwards(code), page).min(most);
        }
        _ => {}
    }
}

fn confirm(app: &mut App, code: KeyCode) {
    let mode = std::mem::replace(&mut app.mode, Mode::List);
    if let (Mode::Confirm(task, _), KeyCode::Char('y')) = (mode, code) {
        app.perform(task);
    }
}

impl App {
    /// Обрабатывает нажатие клавиши.
    pub(crate) fn press(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.done = true;
            return;
        }
        match self.mode {
            Mode::List => {
                self.message.clear();
                let name = key_name(key);
                if !self.custom(&name)
                    && let Some(found) = action(self, &name)
                {
                    act(self, &found);
                }
            }
            Mode::Filter => filter(self, key.code),
            Mode::Confirm(..) => confirm(self, key.code),
            Mode::Text { .. } | Mode::Logs { .. } => output(self, key.code),
        }
    }
}
