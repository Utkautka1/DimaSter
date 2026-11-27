//! Кадр экрана контейнеров. Всё пришедшее от Docker проходит через `fitted`,
//! поэтому управляющие символы из имён и логов не командуют терминалом.

use dimaster_config::{ViewColor, ViewColumn};
use dimaster_i18n::{t, tr};
use dimaster_view::{aligned, fitted, sgr};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Mode};
use crate::kind::{Item, KINDS, Kind};

const RESET: &str = "\x1b[0m";

fn title(kind: Kind) -> String {
    tr(match kind {
        Kind::Containers => "Контейнеры",
        Kind::Compose => "Проекты",
        Kind::Images => "Образы",
        Kind::Volumes => "Тома",
        Kind::Networks => "Сети",
    })
}

fn tabs(app: &App, width: usize) -> String {
    let mut plain = format!(" {} │", app.host().label());
    let mut painted = format!("\x1b[1m{plain}{RESET}");
    for kind in KINDS {
        let text = format!(" {} ", title(kind));
        let style = if kind == app.kind { "\x1b[7m" } else { "" };
        plain += &text;
        painted += &format!("{style}{text}{RESET}");
    }
    // в узком окне остаётся строка без оформления, обрезанная по краю
    match plain.width() <= width {
        true => painted,
        false => fitted(&plain, width),
    }
}

// значение колонки; CPU и память приходят отдельно
fn value(app: &App, item: &Item, source: usize) -> String {
    let names = app.kind.columns();
    let stats = app.stats.get(&item.id);
    match names[source].as_str() {
        "cpu" => stats.map(|s| s.0.clone()).unwrap_or_default(),
        "memory" => stats.map(|s| s.1.clone()).unwrap_or_default(),
        _ => item.cells[source].clone(),
    }
}

fn state_color(name: &str, value: &str) -> Option<ViewColor> {
    let index = match value {
        "running" => 2,
        "exited" | "created" => 8,
        _ => 3,
    };
    (name == "state").then_some(ViewColor::Index(index))
}

// текст и цвет ячейки с учётом `format` из `docker.lua`
fn content(app: &App, item: &Item, column: &ViewColumn) -> (String, Option<ViewColor>) {
    let names = app.kind.columns();
    let name = &names[column.source];
    let raw = value(app, item, column.source);
    let record: Vec<(&str, String)> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), value(app, item, i)))
        .collect();
    let color = column.color.or_else(|| state_color(name, &raw));
    match app.config.view.cell(app.kind.name(), name, &raw, &record) {
        Some(Ok((text, own))) => (text, own.or(color)),
        Some(Err(error)) => (error, Some(ViewColor::Index(1))),
        None => (raw, color),
    }
}

// ширина по самому длинному значению, не больше `max_width`
fn widths(app: &App, columns: &[ViewColumn], items: &[&Item]) -> Vec<usize> {
    let most = app.config.view.settings.max_width;
    let natural = |column: &ViewColumn| {
        let values = items.iter().map(|i| value(app, i, column.source).width());
        values
            .max()
            .unwrap_or(0)
            .max(column.title.width())
            .min(most)
    };
    let width = |c: &ViewColumn| c.width.unwrap_or_else(|| natural(c));
    columns.iter().map(width).collect()
}

// склеивает ячейки, пока есть место, и добивает строку пробелами
fn line(
    app: &App,
    style: &str,
    screen: usize,
    cells: impl Iterator<Item = (String, usize, bool, Option<ViewColor>)>,
) -> String {
    let gap = app.config.view.settings.gap;
    let (mut out, mut used) = (style.to_owned(), 0);
    for (text, width, right, color) in cells {
        let width = width.min(screen - used);
        let paint = sgr(color, 38);
        let text = aligned(&text, width, right);
        out += &format!("{paint}{text}{RESET}{style}");
        let space = gap.min(screen - used - width);
        out += &" ".repeat(space);
        used += width + space;
        if used >= screen {
            break;
        }
    }
    out + &" ".repeat(screen - used) + RESET
}

fn row(
    app: &App,
    item: &Item,
    selected: bool,
    shape: &[(ViewColumn, usize)],
    screen: usize,
) -> String {
    let colors = app.config.view.settings.colors;
    let style = match (selected, colors.selected) {
        (false, _) => sgr(colors.text, 38),
        (true, Some(color)) => sgr(Some(color), 48) + &sgr(colors.selected_text, 38),
        (true, None) => "\x1b[7m".to_owned(),
    };
    let cells = shape.iter().map(|(column, width)| {
        let (text, color) = content(app, item, column);
        (text, *width, column.right, color)
    });
    line(app, &style, screen, cells)
}

// прокручивает список так, чтобы выбранная строка была видна
fn scroll(app: &mut App, rows: usize) {
    app.page = rows.max(1);
    let lowest = (app.selected + 1).saturating_sub(app.page);
    app.top = app.top.clamp(lowest, app.selected);
}

fn list(app: &mut App, screen: usize, height: usize) -> Vec<String> {
    scroll(app, height.saturating_sub(1));
    let items = app.visible();
    let columns = app
        .config
        .view
        .columns(app.kind.name(), &app.kind.columns());
    let widths = widths(app, &columns, &items);
    let shape: Vec<_> = columns.into_iter().zip(widths).collect();
    let colors = app.config.view.settings.colors;
    let style = format!("\x1b[1m{}", sgr(colors.header, 38));
    let titles = shape
        .iter()
        .map(|(c, w)| (c.title.clone(), *w, c.right, None));
    let mut lines = vec![line(app, &style, screen, titles)];
    let shown = items.iter().enumerate().skip(app.top).take(app.page);
    lines.extend(shown.map(|(at, item)| row(app, item, at == app.selected, &shape, screen)));
    if items.is_empty() {
        let empty = if app.loading {
            tr("Загрузка…")
        } else {
            tr("Пусто")
        };
        lines.push(fitted(&empty, screen));
    }
    lines
}

fn body(app: &mut App, width: usize, height: usize) -> Vec<String> {
    app.page = height.max(1);
    let plain = |text: &String| fitted(text, width);
    match &app.mode {
        Mode::Text { lines, scroll } => {
            lines.iter().skip(*scroll).take(height).map(plain).collect()
        }
        Mode::Logs { tail, back } => {
            let end = tail.lines.len().saturating_sub(*back);
            let start = end.saturating_sub(height);
            tail.lines.range(start..end).map(plain).collect()
        }
        _ => list(app, width, height),
    }
}

fn hint(app: &App) -> String {
    if !app.message.is_empty() {
        return app.message.replace('\n', " ");
    }
    match &app.mode {
        Mode::Filter => format!("/{}▏", app.filter),
        Mode::Confirm(_, question) => t!(
            "{question}  y — да, любая другая — нет",
            question = question
        ),
        Mode::Text { .. } => tr("j/k — прокрутка · q — назад"),
        Mode::Logs { back: 0, .. } => tr("логи идут · k — выше · q — назад"),
        Mode::Logs { .. } => tr("логи отмотаны · G — к свежим · q — назад"),
        Mode::List if app.kind == Kind::Containers => tr(
            "s старт/стоп · r перезапуск · l логи · Enter shell · d удалить · h сервер · q выход",
        ),
        Mode::List => tr("Tab вкладки · / фильтр · d удалить · D очистить · h сервер · q выход"),
    }
}

// нижняя строка: своя функция `status` или подсказка
fn status(app: &App, width: usize) -> String {
    let info = [
        ("host", app.host().label().to_owned()),
        ("tab", app.kind.name().to_owned()),
        ("rows", app.visible().len().to_string()),
        ("message", app.message.clone()),
    ];
    let own = matches!(app.mode, Mode::List).then(|| app.config.view.status(&info));
    let text = match own.flatten() {
        Some(result) => result.unwrap_or_else(|error| error),
        None => hint(app),
    };
    let color = sgr(app.config.view.settings.colors.status, 38);
    format!("\x1b[7m{color}{}{RESET}", fitted(&text, width))
}

/// Все строки экрана `width` × `height`.
pub(crate) fn frame(app: &mut App, width: usize, height: usize) -> Vec<String> {
    let inner = height.saturating_sub(2);
    let mut lines = vec![tabs(app, width)];
    lines.extend(body(app, width, inner));
    lines.resize(inner + 1, String::new());
    lines.push(status(app, width));
    lines
}
