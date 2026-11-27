//! Постраничное чтение результатов.

use dimaster_data::{Database, Format, Options, Value};

fn database() -> Database {
    let mut db = Database::new().expect("база");
    let text = "name,age\nann,31\nbob,25\ncid,40\n";
    db.load_bytes("people", Format::Csv, text.as_bytes(), &Options::default())
        .expect("csv");
    db
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

#[test]
fn rows_are_read_by_number() {
    let db = database();
    let table = db.open_table("people").expect("таблица");
    assert_eq!(table.len(), 3);
    let found = db.fetch(&table, &[2, 0, 9]).expect("строки");
    assert_eq!(found[0][0], text("cid"));
    assert_eq!(found[1][0], text("ann"));
    assert_eq!(found[2], [Value::Null, Value::Null]);
}

#[test]
fn query_result_keeps_order_and_names() {
    let db = database();
    let sql = "SELECT name, name FROM people ORDER BY age";
    let result = db.open(sql).expect("запрос");
    assert_eq!(result.columns, ["name", "name"]);
    let mut seen = Vec::new();
    db.scan(&result, |row| {
        seen.push(row[0].clone());
        Ok(())
    })
    .expect("обход");
    assert_eq!(seen, [text("bob"), text("ann"), text("cid")]);
    db.close(result);
    assert_eq!(db.tables().expect("таблицы"), ["people"]);
}

#[test]
fn gaps_views_and_pragmas_still_work() {
    let db = database();
    db.query("DELETE FROM people WHERE name = 'ann'")
        .expect("delete");
    let table = db.open_table("people").expect("таблица");
    assert_eq!(db.fetch(&table, &[0]).expect("строка")[0][0], text("bob"));
    db.query("CREATE VIEW old AS SELECT * FROM people WHERE age > 30")
        .expect("view");
    let view = db.open_table("old").expect("представление");
    assert_eq!(view.len(), 1);
    assert_eq!(
        db.open("PRAGMA table_info(people)").expect("pragma").len(),
        2
    );
}
