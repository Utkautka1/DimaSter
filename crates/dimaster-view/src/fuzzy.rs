//! Нечёткий поиск по строкам: буквы запроса должны идти в строке по порядку,
//! чем плотнее — тем выше строка в списке.

use dimaster_data::Table;

// длина самого раннего отрезка, содержащего буквы по порядку
fn span(text: &str, query: &[char]) -> Option<usize> {
    let mut wanted = query.iter();
    let mut next = wanted.next()?;
    let mut start = None;
    for (at, ch) in text.chars().flat_map(char::to_lowercase).enumerate() {
        if ch == *next {
            let from = *start.get_or_insert(at);
            match wanted.next() {
                Some(following) => next = following,
                None => return Some(at - from + 1),
            }
        }
    }
    None
}

/// Номера подходящих строк; пустой запрос оставляет все по порядку.
pub(crate) fn filter(table: &Table, query: &str) -> Vec<usize> {
    let query: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    if query.is_empty() {
        return (0..table.rows.len()).collect();
    }
    let score = |(index, row): (usize, &Vec<_>)| {
        let cells = row.iter().map(dimaster_data::Value::text);
        // ячейка целиком важнее совпадения, размазанного по строке
        let best = cells.clone().filter_map(|c| span(&c, &query)).min();
        let joined = || span(&cells.collect::<Vec<_>>().join(" "), &query).map(|s| s + 10_000);
        best.or_else(joined).map(|s| (s, index))
    };
    let mut found: Vec<_> = table.rows.iter().enumerate().filter_map(score).collect();
    found.sort_unstable();
    found.into_iter().map(|(_, index)| index).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tighter_matches_come_first() {
        let rows = ["a-x-b", "zzz", "ab"].map(|s| vec![s.to_owned()]).to_vec();
        let table = Table::from_text(vec!["c".into()], rows);
        assert_eq!(filter(&table, "AB"), [2, 0]);
        assert_eq!(filter(&table, "").len(), 3);
    }
}
