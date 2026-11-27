//! LevelDB: пары читаются, а папка с базой не меняется.

use std::path::Path;

use dimaster_data::{Database, Options, Value, is_leveldb};
use rusty_leveldb::DB;

fn listing(path: &Path) -> Vec<(std::ffi::OsString, Vec<u8>)> {
    let entries = std::fs::read_dir(path).expect("каталог").flatten();
    let file = |e: std::fs::DirEntry| (e.file_name(), std::fs::read(e.path()).expect("файл"));
    let mut found: Vec<_> = entries.map(file).collect();
    found.sort();
    found
}

#[test]
fn reads_pairs_and_leaves_the_folder_alone() {
    let path = std::env::temp_dir().join(format!("dimaster-ldb-test-{}", std::process::id()));
    let mut source = DB::open(&path, rusty_leveldb::Options::default()).expect("создание");
    source.put(b"name", b"ann").expect("запись");
    source.put(b"raw", &[0, 255]).expect("запись");
    source.flush().expect("диск");
    drop(source);
    let before = listing(&path);
    assert!(is_leveldb(&path) && !is_leveldb(&std::env::temp_dir()));
    let mut db = Database::new().expect("база");
    let added = db.load(&path, &Options::default()).expect("чтение");
    let rows = db.table(&added[0]).expect("таблица").rows;
    assert_eq!(listing(&path), before);
    std::fs::remove_dir_all(&path).expect("уборка");
    assert_eq!(rows[0][1], Value::Text("ann".into()));
    assert_eq!(rows[1][1], Value::Text("0x00ff".into()));
}
