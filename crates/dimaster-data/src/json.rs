//! JSON и JSON Lines. Колонки — объединение ключей всех записей; вложенные
//! объекты и массивы остаются в ячейке текстом JSON.

use serde_json::Value as Json;

use crate::{DataError, Table, Value};

fn cell(value: &Json) -> Value {
    match value {
        Json::Null => Value::Null,
        Json::Bool(flag) => Value::Text(flag.to_string()),
        Json::Number(number) => match number.as_i64() {
            Some(whole) => Value::Int(whole),
            None => number.as_f64().map_or(Value::Null, Value::Real),
        },
        Json::String(text) => Value::Text(text.clone()),
        nested => Value::Text(nested.to_string()),
    }
}

// не объект — это запись из одной колонки `value`
fn fields(record: &Json) -> Vec<(&str, &Json)> {
    match record {
        Json::Object(map) => map.iter().map(|(k, v)| (k.as_str(), v)).collect(),
        other => vec![("value", other)],
    }
}

fn table(records: &[Json]) -> Table {
    let mut columns: Vec<String> = Vec::new();
    let mut index = std::collections::HashMap::new();
    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        let mut row = vec![Value::Null; columns.len()];
        for (key, value) in fields(record) {
            let at = *index.entry(key.to_owned()).or_insert_with(|| {
                columns.push(key.to_owned());
                columns.len() - 1
            });
            row.resize(columns.len(), Value::Null);
            row[at] = cell(value);
        }
        rows.push(row);
    }
    Table::new(columns, rows)
}

pub(crate) fn parse(text: &str) -> Result<Table, DataError> {
    Ok(match serde_json::from_str(text)? {
        Json::Array(records) => table(&records),
        single => table(&[single]),
    })
}

pub(crate) fn parse_lines(text: &str) -> Result<Table, DataError> {
    let lines = text.lines().filter(|l| !l.trim().is_empty());
    let records: Result<Vec<Json>, _> = lines.map(serde_json::from_str).collect();
    Ok(table(&records?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_merged_in_order() {
        let table = parse_lines("{\"b\":1}\n\n{\"a\":[1,2],\"b\":2}\n").expect("jsonl");
        assert_eq!(table.columns, ["b", "a"]);
        assert_eq!(table.rows[0][1], Value::Null);
        assert_eq!(table.rows[1][1], Value::Text("[1,2]".into()));
    }
}
