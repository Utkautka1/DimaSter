//! Собственная запись terminfo.
//!
//! По умолчанию `TERM` остаётся `xterm-256color`: на удалённых серверах записи
//! `xterm-dimaster` нет, и по SSH ломались бы программы. Своя запись включается
//! `term = "xterm-dimaster"` в конфиге.

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

const SOURCE: &str = include_str!("../../../assets/dimaster.terminfo");
const NAME: &str = "xterm-dimaster";
const FALLBACK: &str = "xterm-256color";

/// Значение `TERM` и каталог terminfo, если нужен свой.
///
/// `configured` берётся из конфига; `DIMASTER_TERM` имеет приоритет.
pub(crate) fn select(configured: &str) -> (String, Option<PathBuf>) {
    let wanted = std::env::var("DIMASTER_TERM").unwrap_or_else(|_| configured.to_owned());
    if wanted != NAME {
        return (wanted, None);
    }
    match install() {
        Ok(directory) => (wanted, Some(directory)),
        Err(e) => {
            log::warn!("Не удалось установить terminfo, используем {FALLBACK}: {e}");
            (FALLBACK.to_owned(), None)
        }
    }
}

fn data_directory() -> io::Result<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".local/share")))
        .ok_or_else(|| io::Error::other("не заданы XDG_DATA_HOME и HOME"))?;
    Ok(base.join("dimaster/terminfo"))
}

/// Компилирует запись через `tic`; повторный вызов просто перезаписывает её.
fn install() -> io::Result<PathBuf> {
    let directory = data_directory()?;
    std::fs::create_dir_all(&directory)?;
    let mut tic = Command::new("tic")
        .args(["-x", "-o"])
        .arg(&directory)
        .arg("-")
        .stdin(Stdio::piped())
        .spawn()?;
    if let Some(mut input) = tic.stdin.take() {
        input.write_all(SOURCE.as_bytes())?;
    }
    let status = tic.wait()?;
    if !status.success() {
        return Err(io::Error::other(format!("tic завершился с {status}")));
    }
    Ok(directory)
}
