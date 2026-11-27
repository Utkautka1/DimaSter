//! Таблица в памяти и угадывание типов по тексту ячеек.

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
}

impl Value {
    /// Число из текста; `007` и `+1` остаются строками, чтобы не потерять вид.
    pub fn parse(text: &str) -> Self {
        if text.is_empty() {
            return Self::Null;
        }
        let digits = text.strip_prefix('-').unwrap_or(text);
        let padded = digits.len() > 1 && digits.starts_with('0') && !digits.starts_with("0.");
        if padded || !digits.starts_with(|c: char| c.is_ascii_digit()) {
            return Self::Text(text.to_owned());
        }
        if let Ok(number) = text.parse() {
            return Self::Int(number);
        }
        match text.parse::<f64>() {
            Ok(number) if number.is_finite() => Self::Real(number),
            _ => Self::Text(text.to_owned()),
        }
    }

    /// Как ячейка выглядит на экране; пустая — пустая строка.
    pub fn text(&self) -> String {
        match self {
            Self::Null => String::new(),
            Self::Int(number) => number.to_string(),
            Self::Real(number) => number.to_string(),
            Self::Text(text) => text.clone(),
        }
    }
}

/// Строки всегда той же длины, что и список колонок.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Table {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

// пустые и повторяющиеся имена SQL не примет
fn unique(names: Vec<String>) -> Vec<String> {
    let mut taken: Vec<String> = Vec::with_capacity(names.len());
    for (index, name) in names.into_iter().enumerate() {
        let name = match name.trim() {
            "" => format!("column_{}", index + 1),
            text => text.to_owned(),
        };
        let clashes = |candidate: &String| taken.iter().any(|t| t.eq_ignore_ascii_case(candidate));
        let mut candidate = name.clone();
        let mut number = 2;
        while clashes(&candidate) {
            candidate = format!("{name}_{number}");
            number += 1;
        }
        taken.push(candidate);
    }
    taken
}

impl Table {
    /// Таблица из уже типизированных строк; короткие строки дополняются.
    pub fn new(columns: Vec<String>, mut rows: Vec<Vec<Value>>) -> Self {
        let width = rows
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(0)
            .max(columns.len());
        let mut columns = columns;
        columns.resize(width, String::new());
        for row in &mut rows {
            row.resize(width, Value::Null);
        }
        Self {
            name: String::new(),
            columns: unique(columns),
            rows,
        }
    }

    /// Таблица из текста: тип колонки общий для всех её ячеек.
    pub fn from_text(columns: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        let parsed = rows
            .iter()
            .map(|r| r.iter().map(|c| Value::parse(c)).collect());
        let mut table = Self::new(columns, parsed.collect());
        for column in 0..table.columns.len() {
            let mixed = table
                .rows
                .iter()
                .any(|r| matches!(r[column], Value::Text(_)));
            if mixed {
                table.keep_text(column, &rows);
            }
        }
        table
    }

    // в текстовой колонке числа показываются так, как были записаны
    fn keep_text(&mut self, column: usize, source: &[Vec<String>]) {
        for (row, original) in self.rows.iter_mut().zip(source) {
            if let Some(text) = original.get(column).filter(|t| !t.is_empty()) {
                row[column] = Value::Text(text.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn types_are_per_column() {
        let rows = vec![text(&["1", "1", "007"]), text(&["2.5", "x", ""])];
        let table = Table::from_text(text(&["a", "a", ""]), rows);
        assert_eq!(table.columns, ["a", "a_2", "column_3"]);
        assert_eq!(table.rows[0][0], Value::Int(1));
        assert_eq!(table.rows[0][1], Value::Text("1".into()));
        assert_eq!(table.rows[0][2], Value::Text("007".into()));
        assert_eq!(table.rows[1][2], Value::Null);
    }
}
