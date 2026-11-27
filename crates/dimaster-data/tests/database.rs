//! Сквозные проверки: файлы разных форматов, `JOIN` между ними и защита диска.

use std::path::PathBuf;

use dimaster_data::{Database, Format, Options, Value};

fn directory(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("dimaster-data-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&path).expect("каталог");
    path
}

#[test]
fn joins_files_of_different_formats() {
    let dir = directory("join");
    std::fs::write(dir.join("users.csv"), "id,name\n1,ann\n2,bob\n").expect("csv");
    std::fs::write(dir.join("pay.json"), "[{\"user\":2,\"sum\":5.5}]").expect("json");
    let mut db = Database::new().expect("база");
    for file in ["users.csv", "pay.json"] {
        db.load(&dir.join(file), &Options::default()).expect("файл");
    }
    let found = db.query("SELECT name, sum FROM users JOIN pay ON user = id");
    std::fs::remove_dir_all(&dir).expect("уборка");
    let row = found.expect("запрос").rows.remove(0);
    assert_eq!(row, [Value::Text("bob".into()), Value::Real(5.5)]);
}

#[test]
fn dump_loads_and_cannot_touch_disk() {
    let mut db = Database::new().expect("база");
    let options = Options::default();
    let dump = "CREATE TABLE t(a); INSERT INTO t VALUES (1);";
    let added = db
        .load_bytes("x", Format::Sql, dump.as_bytes(), &options)
        .expect("дамп");
    assert_eq!(added, ["t"]);
    let target = std::env::temp_dir().join("dimaster-data-must-not-exist.db");
    let attack = format!(
        "ATTACH '{}' AS evil; CREATE TABLE evil.x(a);",
        target.display()
    );
    assert!(
        db.load_bytes("x", Format::Sql, attack.as_bytes(), &options)
            .is_err()
    );
    assert!(!target.exists());
}

#[test]
fn sqlite_file_is_copied() {
    let dir = directory("copy");
    let file = dir.join("shop.db");
    let source = rusqlite::Connection::open(&file).expect("база");
    source
        .execute_batch("CREATE TABLE goods(n INTEGER); INSERT INTO goods VALUES (3);")
        .expect("данные");
    drop(source);
    let mut db = Database::new().expect("база");
    let added = db.load(&file, &Options::default());
    let table = db.table("goods");
    std::fs::remove_dir_all(&dir).expect("уборка");
    assert_eq!(added.expect("копия"), ["goods"]);
    assert_eq!(table.expect("таблица").rows, [[Value::Int(3)]]);
}
