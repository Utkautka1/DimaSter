//! Папки LevelDB: все пары «ключ — значение» одной таблицей.
//!
//! Открытие LevelDB само пишет в папку (журнал, новый манифест, сжатие),
//! поэтому открывается копия во временном каталоге, а оригинал только читается.

use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use rusty_leveldb::{DB, LdbIterator, Options};

use crate::{DataError, Table, Value};

/// Похожа ли папка на базу LevelDB: файл `CURRENT` называет манифест.
pub fn is_leveldb(path: &Path) -> bool {
    let mut start = [0; 9];
    let current = std::fs::File::open(path.join("CURRENT"));
    let read = current.and_then(|mut f| std::io::Read::read_exact(&mut f, &mut start));
    read.is_ok() && &start == b"MANIFEST-"
}

/// Временная копия базы, закрытая от других пользователей; удаляется сама.
struct Copy(PathBuf);

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn copy(source: &Path) -> std::io::Result<Copy> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let number = NEXT.fetch_add(1, Ordering::Relaxed);
    let name = format!("dimaster-leveldb-{}-{number}", std::process::id());
    let target = std::env::temp_dir().join(name);
    std::fs::DirBuilder::new().mode(0o700).create(&target)?;
    let target = Copy(target);
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        // ссылки и вложенные папки к базе не относятся
        if entry.file_type()?.is_file() && entry.file_name() != "LOCK" {
            std::fs::copy(entry.path(), target.0.join(entry.file_name()))?;
        }
    }
    Ok(target)
}

// текст как есть, двоичное — шестнадцатерично
fn value(bytes: &[u8]) -> Value {
    let readable = |t: &str| t.chars().all(|c| !c.is_control() || c.is_whitespace());
    match std::str::from_utf8(bytes) {
        Ok(text) if readable(text) => Value::Text(text.to_owned()),
        _ => {
            let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            Value::Text(format!("0x{hex}"))
        }
    }
}

fn failed(error: impl std::fmt::Display) -> DataError {
    DataError::Parse(error.to_string())
}

pub(crate) fn read(path: &Path) -> Result<Table, DataError> {
    let copy = copy(path)?;
    let options = Options {
        create_if_missing: false,
        ..Options::default()
    };
    let mut database = DB::open(&copy.0, options).map_err(failed)?;
    let mut entries = database.new_iter().map_err(failed)?;
    let mut rows = Vec::new();
    while let Some((key, stored)) = LdbIterator::next(&mut entries) {
        rows.push(vec![value(&key), value(&stored)]);
    }
    Ok(Table::new(vec!["key".into(), "value".into()], rows))
}
