//! Нечёткий поиск по строкам: буквы запроса должны идти в строке по порядку,
//! чем плотнее — тем выше строка в списке.

use dimaster_data::{DataError, Database, Rows, Value};

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

// чем меньше, тем выше; `None` — строка не подходит
fn score(row: &[Value], query: &[char]) -> Option<usize> {
    let cells = row.iter().map(Value::text);
    // ячейка целиком важнее совпадения, размазанного по строке
    let best = cells.clone().filter_map(|c| span(&c, query)).min();
    let joined = || span(&cells.collect::<Vec<_>>().join(" "), query).map(|s| s + 10_000);
    best.or_else(joined)
}

/// Номера подходящих строк; `None` — пустой запрос, подходят все.
pub(crate) fn filter(
    db: &Database,
    data: &Rows,
    query: &str,
) -> Result<Option<Vec<usize>>, DataError> {
    let query: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    if query.is_empty() {
        return Ok(None);
    }
    let (mut found, mut index) = (Vec::new(), 0);
    db.scan(data, |row| {
        found.extend(score(row, &query).map(|s| (s, index)));
        index += 1;
        Ok(())
    })?;
    found.sort_unstable();
    Ok(Some(found.into_iter().map(|(_, index)| index).collect()))
}

#[cfg(test)]
mod tests {
    use dimaster_data::{Format, Options};

    use super::*;

    #[test]
    fn tighter_matches_come_first() {
        let mut db = Database::new().expect("база");
        db.load_bytes(
            "t",
            Format::Csv,
            b"c\na-x-b\nzzz\nab\n",
            &Options::default(),
        )
        .expect("csv");
        let data = db.open_table("t").expect("таблица");
        let found = filter(&db, &data, "AB").expect("поиск");
        assert_eq!(found, Some(vec![2, 0]));
        assert_eq!(filter(&db, &data, "").expect("поиск"), None);
    }
}
