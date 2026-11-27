//! Запрет `ATTACH` и пределы запроса.

use std::time::Duration;

use dimaster_data::{DataError, Database, Limits, Options};

#[test]
fn attach_stays_forbidden_after_broken_file() {
    let name = format!("dimaster-broken-{}.sqlite", std::process::id());
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, "это не база ".repeat(50)).expect("файл");
    let mut db = Database::new().expect("база");
    assert!(db.load(&path, &Options::default()).is_err());
    assert!(db.query("ATTACH ':memory:' AS other").is_err());
    std::fs::remove_file(path).expect("уборка");
}

#[test]
fn endless_queries_are_stopped() {
    const ENDLESS: &str = "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n)";
    let mut db = Database::new().expect("база");
    db.set_limits(Limits {
        time: Duration::from_millis(200),
        bytes: 1 << 16,
    });
    let rows = db.query(&format!("{ENDLESS} SELECT x FROM n"));
    assert!(matches!(rows, Err(DataError::TooLarge)));
    assert!(
        db.query(&format!("{ENDLESS} SELECT count(*) FROM n"))
            .is_err()
    );
    assert!(db.query("SELECT 1").is_ok());
}
