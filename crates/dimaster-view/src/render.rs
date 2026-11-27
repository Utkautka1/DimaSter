//! Кадр целиком: строки с кодами ANSI под размер окна.
//!
//! Управляющие символы из данных заменяются: иначе файл мог бы командовать
//! терминалом.

use unicode_width::UnicodeWidthChar;

use dimaster_config::{ViewColor, ViewColumn, ViewSettings};
use dimaster_data::Value;
use dimaster_i18n::{t, tr};

use crate::app::{App, Mode};
use crate::plot;
use crate::sheet::Sheet;

const RESET: &str = "\x1b[0m";

/// Ровно `width` знакомест; лишнее обрывается многоточием.
pub(crate) fn fitted(text: &str, width: usize) -> String {
    let safe = text.chars().map(|c| if c.is_control() { ' ' } else { c });
    let mut out = String::new();
    let mut used = 0;
    let total: usize = safe.clone().map(|c| c.width().unwrap_or(0)).sum();
    let limit = if total > width {
        width.saturating_sub(1)
    } else {
        width
    };
    for ch in safe {
        let size = ch.width().unwrap_or(0);
        if used + size > limit {
            break;
        }
        out.push(ch);
        used += size;
    }
    if total > width && width > 0 {
        out.push('…');
        used += 1;
    }
    out + &" ".repeat(width - used)
}

// SGR цвета текста или, с `base` = 48, фона
fn sgr(color: Option<ViewColor>, base: u8) -> String {
    match color {
        None => String::new(),
        Some(ViewColor::Index(i)) => format!("\x1b[{base};5;{i}m"),
        Some(ViewColor::Rgb(c)) => {
            format!(
                "\x1b[{base};2;{};{};{}m",
                c >> 16 & 255,
                c >> 8 & 255,
                c & 255
            )
        }
    }
}

fn size(text: &str) -> usize {
    text.chars().map(|c| c.width().unwrap_or(0)).sum()
}

// естественные ширины по заголовку и первым строкам
fn widths(sheet: &Sheet, view: &ViewSettings, fit: bool, screen: usize) -> Vec<usize> {
    let count = sheet.columns.len();
    let share = (screen.saturating_sub(count * view.gap) / count.max(1)).max(3);
    let cap = match (count, fit) {
        // единственной колонке (например логу) отдаётся весь экран
        (1, _) => screen.saturating_sub(view.gap),
        (_, true) => share.min(view.max_width),
        (_, false) => view.max_width,
    };
    let natural = |column: &ViewColumn| {
        let rows = sheet.sample.iter();
        let longest = rows.map(|r| size(&r[column.source].text())).max();
        longest
            .unwrap_or(0)
            .max(size(&column.title))
            .max(1)
            .min(cap)
    };
    let width = |c: &ViewColumn| c.width.unwrap_or_else(|| natural(c));
    sheet.columns.iter().map(width).collect()
}

// колонки, помещающиеся начиная с `left`, с курсором среди них
fn visible(sheet: &mut Sheet, widths: &[usize], gap: usize, screen: usize) -> Vec<(usize, usize)> {
    sheet.left = sheet.left.min(sheet.column);
    loop {
        let mut used = 0;
        let mut found = Vec::new();
        for (index, &width) in widths.iter().enumerate().skip(sheet.left) {
            if used >= screen {
                break;
            }
            found.push((index, width.min(screen - used)));
            used += width + gap;
        }
        let whole = |&(i, w): &(usize, usize)| i == sheet.column && w == widths[i];
        if found.iter().any(whole) || sheet.left >= sheet.column {
            return found;
        }
        sheet.left += 1;
    }
}

// выравнивание вправо только для того, что помещается целиком
fn aligned(text: &str, width: usize, right: bool) -> String {
    match right && size(text) < width {
        true => " ".repeat(width - size(text)) + &fitted(text, size(text)),
        false => fitted(text, width),
    }
}

/// Всё, что нужно для строк одной таблицы.
struct Grid<'a> {
    app: &'a App,
    sheet: &'a Sheet,
    cells: Vec<(usize, usize)>,
    // ширина колонки номеров вместе с отступом; 0 — номеров нет
    numbers: usize,
    screen: usize,
    // только строки экрана, начиная с `sheet.top`
    page: Vec<Vec<Value>>,
}

impl Grid<'_> {
    fn view(&self) -> &ViewSettings {
        &self.app.viewer.settings
    }

    // номер строки своим цветом перед ячейками
    fn number(&self, text: &str, style: &str) -> String {
        if self.numbers == 0 {
            return String::new();
        }
        let color = sgr(self.view().colors.numbers, 38);
        let text = aligned(text, self.numbers - 1, true);
        format!("{color}{text}{RESET}{style} ")
    }

    // собирает строку из ячеек и добивает пробелами до края
    fn line(
        &self,
        number: &str,
        style: &str,
        cell: impl Fn(&ViewColumn, usize) -> String,
    ) -> String {
        let mut out = format!("{style}{}", self.number(number, style));
        let mut used = self.numbers;
        for &(index, width) in &self.cells {
            let gap = self
                .view()
                .gap
                .min(self.screen.saturating_sub(used + width));
            out += &cell(&self.sheet.columns[index], width);
            out += &" ".repeat(gap);
            used += width + gap;
        }
        out + &" ".repeat(self.screen.saturating_sub(used)) + RESET
    }

    fn header(&self) -> String {
        let style = format!("\x1b[1m{}", sgr(self.view().colors.header, 38));
        self.line("#", &style, |c, width| aligned(&c.title, width, c.right))
    }

    fn row_style(&self, at: usize) -> String {
        let colors = self.view().colors;
        let text = sgr(colors.text, 38);
        if at != self.sheet.row {
            let zebra = colors.zebra.filter(|_| at % 2 == 1);
            return text + &sgr(zebra, 48);
        }
        match colors.selected {
            Some(color) => sgr(Some(color), 48) + &sgr(colors.selected_text, 38),
            None => format!("{text}\x1b[7m"),
        }
    }

    // текст и цвет ячейки с учётом `format`; ошибка функции видна в ячейке
    fn content(&self, column: &ViewColumn, row: &[Value]) -> (String, Option<ViewColor>) {
        let raw = row[column.source].text();
        let names = &self.sheet.data.columns;
        let record: Vec<(&str, String)> = names
            .iter()
            .map(String::as_str)
            .zip(row.iter().map(Value::text))
            .collect();
        let viewer = &self.app.viewer;
        match viewer.cell(&self.sheet.title, &names[column.source], &raw, &record) {
            Some(Ok((text, color))) => (text, color.or(column.color)),
            Some(Err(error)) => (error, Some(ViewColor::Index(1))),
            None => (raw, column.color),
        }
    }

    fn cell(&self, column: &ViewColumn, width: usize, row: &[Value], style: &str) -> String {
        let (text, color) = self.content(column, row);
        let text = aligned(&text, width, column.right);
        match color {
            Some(_) => format!("{}{text}{RESET}{style}", sgr(color, 38)),
            None => text,
        }
    }

    fn row(&self, at: usize) -> String {
        let row = &self.page[at - self.sheet.top];
        let style = self.row_style(at);
        let number = (at + 1).to_string();
        self.line(&number, &style, |c, width| self.cell(c, width, row, &style))
    }
}

// ширина номеров и видимые колонки; прокручивает таблицу к курсору
fn arrange(app: &mut App, screen: usize, rows: usize) -> (usize, Vec<(usize, usize)>) {
    let view = &app.viewer.settings;
    let sheet = &mut app.sheets[app.active];
    let numbers = match view.row_numbers {
        true => sheet.len().to_string().len() + 1,
        false => 0,
    };
    let room = screen.saturating_sub(numbers);
    let widths = widths(sheet, view, app.fit, room);
    sheet.top = sheet
        .top
        .clamp((sheet.row + 1).saturating_sub(rows), sheet.row);
    // курсор мог остаться за концом укоротившегося результата
    sheet.top = sheet.top.min(sheet.len());
    (numbers, visible(sheet, &widths, view.gap, room))
}

fn grid(app: &mut App, screen: usize, rows: usize) -> Vec<String> {
    let (numbers, cells) = arrange(app, screen, rows);
    let sheet = &app.sheets[app.active];
    let grid = Grid {
        app,
        sheet,
        cells,
        numbers,
        screen,
        page: sheet.page(&app.db, sheet.top, rows),
    };
    let shown = sheet.top..sheet.top + grid.page.len();
    let header = grid.view().header.then(|| grid.header());
    header
        .into_iter()
        .chain(shown.map(|at| grid.row(at)))
        .collect()
}

// запись столбиком или объектом JSON
fn record(app: &App, as_json: bool, screen: usize) -> Vec<String> {
    let pairs = app.sheets[app.active].record(&app.db);
    let quoted = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
    if as_json {
        let body = pairs
            .iter()
            .map(|(k, v)| format!("  {}: {}", quoted(k), quoted(v)));
        let mut out = vec!["{".to_owned()];
        out.extend(body.map(|l| fitted(&(l + ","), screen)));
        out.push("}".to_owned());
        return out;
    }
    let mut out = Vec::new();
    for (name, value) in pairs {
        out.push(format!("\x1b[1;34m{}{RESET}", fitted(name, screen)));
        out.extend(value.lines().map(|l| fitted(&format!("  {l}"), screen)));
    }
    out
}

fn tab_list(app: &App, screen: usize) -> Vec<String> {
    let entry = |(index, sheet): (usize, &Sheet)| {
        let style = if index == app.active { "\x1b[7m" } else { "" };
        let text = format!(" {} ({})", sheet.title, sheet.data.len());
        format!("{style}{}{RESET}", fitted(&text, screen))
    };
    app.sheets.iter().enumerate().map(entry).collect()
}

// сведения для строки состояния: row, rows, tab, tabs, title, column, sql
fn details(app: &App) -> [(&'static str, String); 7] {
    let sheet = &app.sheets[app.active];
    let column = sheet
        .columns
        .get(sheet.column)
        .map_or("", |c| c.title.as_str());
    let row = (sheet.row + 1).min(sheet.len());
    [
        ("title", sheet.title.clone()),
        ("row", row.to_string()),
        ("rows", sheet.len().to_string()),
        ("column", column.to_owned()),
        ("tab", (app.active + 1).to_string()),
        ("tabs", app.sheets.len().to_string()),
        ("sql", sheet.sql.clone()),
    ]
}

// где мы находимся: своя функция `status` или обычный вид
fn position(app: &App) -> String {
    let info = details(app);
    if let Some(own) = app.viewer.status(&info) {
        return own.unwrap_or_else(|error| error);
    }
    t!(
        "{title} [{tab}/{tabs}] · строка {row}/{rows} · {column}",
        title = &info[0].1,
        tab = &info[4].1,
        tabs = &info[5].1,
        row = &info[1].1,
        rows = &info[2].1,
        column = &info[3].1
    )
}

// нижняя строка: ввод, сообщение или где мы находимся
fn status(app: &App, screen: usize) -> String {
    let text = match app.mode {
        Mode::Command => format!(":{}▏", app.input),
        Mode::Search => format!("/{}▏", app.input),
        _ if !app.message.is_empty() => app.message.clone(),
        Mode::Record(_) => tr("f — вид · c — копировать · Esc — назад"),
        Mode::Table | Mode::Tabs => position(app),
        Mode::Plot => tr("Esc — назад к таблице"),
    };
    let color = sgr(app.viewer.settings.colors.status, 38);
    format!("\x1b[7m{color}{}{RESET}", fitted(&text, screen))
}

fn chart(app: &App, width: usize, height: usize) -> Vec<String> {
    let sheet = &app.sheets[app.active];
    let draw = |p| plot::draw(p, &app.db, sheet, width, height);
    app.plot.as_ref().map(draw).unwrap_or_default()
}

/// Все строки экрана `width` × `height`.
pub(crate) fn frame(app: &mut App, width: usize, height: usize) -> Vec<String> {
    let body = height.saturating_sub(1);
    let header = usize::from(app.viewer.settings.header);
    app.page = body.saturating_sub(header).max(1);
    let mut lines = match app.mode {
        Mode::Record(as_json) => {
            let all = record(app, as_json, width);
            app.scroll = app.scroll.min(all.len().saturating_sub(1));
            all.into_iter().skip(app.scroll).collect()
        }
        Mode::Tabs => tab_list(app, width),
        Mode::Plot => chart(app, width, body),
        _ => grid(app, width, app.page),
    };
    lines.resize(body, String::new());
    lines.push(status(app, width));
    lines
}

#[cfg(test)]
mod tests {
    use dimaster_config::Viewer;
    use dimaster_data::{Database, Format, Options};

    use super::*;

    #[test]
    fn lua_changes_the_grid() {
        let mut db = Database::new().expect("база");
        let options = Options::default();
        db.load_bytes("t", Format::Csv, b"a,b\n1,2\n", &options)
            .expect("csv");
        let viewer = Viewer::parse(
            "return { row_numbers = true, columns = { order = { 'b' }, \
             b = { title = 'Second', format = function(v) return v .. '!' end } } }",
        );
        let app = App::new(db).expect("приложение");
        let mut app = app.with(viewer.expect("view.lua"));
        let screen = frame(&mut app, 40, 5).join("\n");
        assert!(screen.contains("Second"));
        assert!(screen.contains("2!"));
    }

    #[test]
    fn cells_are_exact_and_harmless() {
        assert_eq!(fitted("ab", 4), "ab  ");
        assert_eq!(fitted("abcdef", 4), "abc…");
        assert_eq!(fitted("a\x1b]0;x", 6), "a ]0;x");
        assert_eq!(fitted("日本語", 4), "日… ");
    }
}
