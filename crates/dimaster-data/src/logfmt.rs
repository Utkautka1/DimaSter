//! Logfmt: `ключ=значение ключ="с пробелами"` по записи в строке.

use std::iter::Peekable;
use std::str::Chars;

use crate::Table;

// значение в кавычках с `\"` внутри
fn quoted(chars: &mut Peekable<Chars<'_>>) -> String {
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => break,
            '\\' => out.extend(chars.next().map(|c| match c {
                'n' => '\n',
                't' => '\t',
                other => other,
            })),
            other => out.push(other),
        }
    }
    out
}

fn word(chars: &mut Peekable<Chars<'_>>, stop: impl Fn(char) -> bool) -> String {
    let mut out = String::new();
    while let Some(ch) = chars.next_if(|&c| !stop(c)) {
        out.push(ch);
    }
    out
}

// пары одной строки; ключ без `=` получает `true`
fn pairs(line: &str) -> Vec<(String, String)> {
    let mut chars = line.chars().peekable();
    let mut pairs = Vec::new();
    while chars.peek().is_some() {
        let key = word(&mut chars, |c| c == '=' || c.is_whitespace());
        let value = match chars.next() {
            Some('=') if chars.next_if_eq(&'"').is_some() => quoted(&mut chars),
            Some('=') => word(&mut chars, char::is_whitespace),
            _ => "true".to_owned(),
        };
        if !key.is_empty() {
            pairs.push((key, value));
        }
    }
    pairs
}

pub(crate) fn parse(text: &str) -> Table {
    let mut columns: Vec<String> = Vec::new();
    let mut rows = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let mut row = vec![String::new(); columns.len()];
        for (key, value) in pairs(line) {
            let at = columns.iter().position(|c| *c == key).unwrap_or_else(|| {
                columns.push(key);
                row.push(String::new());
                columns.len() - 1
            });
            row[at] = value;
        }
        rows.push(row);
    }
    Table::from_text(columns, rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn quoted_values_and_flags() {
        let table = parse("level=info msg=\"a \\\"b\\\"\" ok\ncode=7\n");
        assert_eq!(table.columns, ["level", "msg", "ok", "code"]);
        assert_eq!(table.rows[0][1], Value::Text("a \"b\"".into()));
        assert_eq!(table.rows[1][3], Value::Int(7));
    }
}
