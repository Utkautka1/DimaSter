//! Двоичные форматы: Parquet, Arrow IPC и Excel.

use std::sync::Arc;

use arrow_array::{ArrayRef, Int64Array, RecordBatch, StringArray};
use dimaster_data::{Database, Format, Options, Value};

fn batch() -> RecordBatch {
    let ids: ArrayRef = Arc::new(Int64Array::from(vec![Some(7), None]));
    let names: ArrayRef = Arc::new(StringArray::from(vec!["a", "007"]));
    RecordBatch::try_from_iter([("id", ids), ("name", names)]).expect("пакет")
}

fn rows(format: Format, bytes: &[u8], table: &str) -> Vec<Vec<Value>> {
    let mut db = Database::new().expect("база");
    db.load_bytes("data", format, bytes, &Options::default())
        .expect("чтение");
    db.table(table).expect("таблица").rows
}

fn expected() -> Vec<Vec<Value>> {
    vec![
        vec![Value::Int(7), Value::Text("a".into())],
        vec![Value::Null, Value::Text("007".into())],
    ]
}

#[test]
fn parquet_round_trip() {
    let batch = batch();
    let mut bytes = Vec::new();
    let mut writer =
        parquet::arrow::ArrowWriter::try_new(&mut bytes, batch.schema(), None).expect("писатель");
    writer.write(&batch).expect("запись");
    writer.close().expect("конец");
    assert_eq!(rows(Format::Parquet, &bytes, "data"), expected());
}

#[test]
fn arrow_file_and_stream() {
    let batch = batch();
    let mut file = Vec::new();
    let mut writer =
        arrow_ipc::writer::FileWriter::try_new(&mut file, &batch.schema()).expect("файл");
    writer.write(&batch).expect("запись");
    writer.finish().expect("конец");
    drop(writer);
    let mut stream = Vec::new();
    let mut writer =
        arrow_ipc::writer::StreamWriter::try_new(&mut stream, &batch.schema()).expect("поток");
    writer.write(&batch).expect("запись");
    writer.finish().expect("конец");
    drop(writer);
    assert_eq!(rows(Format::Arrow, &file, "data"), expected());
    assert_eq!(rows(Format::Arrow, &stream, "data"), expected());
}

#[test]
fn excel_sheets_become_tables() {
    let mut book = rust_xlsxwriter::Workbook::new();
    let sheet = book.add_worksheet().set_name("goods").expect("имя");
    sheet.write(0, 0, "n").expect("ячейка");
    sheet.write(1, 0, 3).expect("ячейка");
    let bytes = book.save_to_buffer().expect("книга");
    assert_eq!(rows(Format::Excel, &bytes, "goods"), [[Value::Int(3)]]);
}
