//! Таблицы из документов: `<table>` в HTML и таблицы с `|` в Markdown.
//!
//! Полный разбор HTML не нужен: достаточно тегов таблицы и текста между ними.

use crate::Table;

fn finish(mut rows: Vec<Vec<String>>) -> Option<Table> {
    if rows.is_empty() {
        return None;
    }
    let columns = rows.remove(0);
    Some(Table::from_text(columns, rows))
}

fn entities(text: &str) -> String {
    let named = [
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&nbsp;", " "),
    ];
    let mut out = text.to_owned();
    for (from, to) in named {
        out = out.replace(from, to);
    }
    // последним, чтобы `&amp;lt;` остался текстом `&lt;`
    out.replace("&amp;", "&")
}

/// Собирает таблицы, получая теги и текст по порядку.
#[derive(Default)]
struct Collector {
    tables: Vec<Table>,
    rows: Vec<Vec<String>>,
    cell: Option<String>,
}

impl Collector {
    fn close_cell(&mut self) {
        let Some(cell) = self.cell.take() else { return };
        let words: Vec<&str> = cell.split_whitespace().collect();
        if let Some(row) = self.rows.last_mut() {
            row.push(entities(&words.join(" ")));
        }
    }

    fn tag(&mut self, name: &str) {
        match name {
            "tr" => {
                self.close_cell();
                self.rows.push(Vec::new());
            }
            "td" | "th" => {
                self.close_cell();
                self.cell = Some(String::new());
            }
            "/td" | "/th" | "/tr" => self.close_cell(),
            "/table" => {
                self.close_cell();
                let mut rows = std::mem::take(&mut self.rows);
                rows.retain(|r| !r.is_empty());
                self.tables.extend(finish(rows));
            }
            "br" => self.text(" "),
            _ => {}
        }
    }

    fn text(&mut self, text: &str) {
        if let Some(cell) = &mut self.cell {
            cell.push_str(text);
        }
    }
}

pub(crate) fn html(text: &str) -> Vec<Table> {
    let mut collector = Collector::default();
    for (index, part) in text.split('<').enumerate() {
        let Some((tag, rest)) = part.split_once('>').filter(|_| index > 0) else {
            collector.text(part);
            continue;
        };
        let name = tag
            .split(|c: char| c.is_whitespace())
            .next()
            .unwrap_or_default();
        collector.tag(&name.to_ascii_lowercase());
        collector.text(rest);
    }
    collector.tables
}

// ячейки строки `| a | b |`; `\|` — это просто черта
fn cells(line: &str) -> Vec<String> {
    let line = line.trim().replace("\\|", "\u{0}");
    let inner = line.strip_prefix('|').unwrap_or(&line);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    inner
        .split('|')
        .map(|c| c.trim().replace('\u{0}', "|"))
        .collect()
}

fn is_rule(line: &str) -> bool {
    line.contains('-') && line.chars().all(|c| "|-: \t".contains(c))
}

pub(crate) fn markdown(text: &str) -> Vec<Table> {
    let lines: Vec<&str> = text.lines().collect();
    let mut tables = Vec::new();
    let mut at = 0;
    while at + 1 < lines.len() {
        if !lines[at].contains('|') || !is_rule(lines[at + 1]) {
            at += 1;
            continue;
        }
        let body = lines[at + 2..].iter().take_while(|l| l.contains('|'));
        let mut rows = vec![cells(lines[at])];
        rows.extend(body.map(|l| cells(l)));
        at += rows.len() + 1;
        tables.extend(finish(rows));
    }
    tables
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn html_tables_with_open_cells() {
        let found = html(
            "<p>x</p><table><tr><th>a<th>b</tr><tr><td><b>1</b></td><td>&amp;lt;</td></tr></table>",
        );
        assert_eq!(found[0].columns, ["a", "b"]);
        assert_eq!(
            found[0].rows[0],
            [Value::Int(1), Value::Text("&lt;".into())]
        );
    }

    #[test]
    fn every_markdown_table() {
        let found = markdown("# t\n| a | b |\n|---|:-:|\n| 1 | x\\|y |\n\ntext\n\nc|d\n-|-\n");
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].rows[0][1], Value::Text("x|y".into()));
        assert!(found[1].rows.is_empty());
    }
}
