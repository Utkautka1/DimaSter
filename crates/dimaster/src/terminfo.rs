//! Собственная запись terminfo.
//!
//! По умолчанию `TERM` остаётся `xterm-256color`: на удалённых серверах записи
//! `xterm-dimaster` нет, и по SSH ломались бы программы. Своя запись включается
//! `term = "xterm-dimaster"` в конфиге.

use dimaster_i18n::t;
use dimaster_i18n::{tr, trf};
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
            log::warn!(
                "{}",
                t!(
                    "Не удалось установить terminfo, используем {fallback}: {error}",
                    fallback = FALLBACK,
                    error = e
                )
            );
            (FALLBACK.to_owned(), None)
        }
    }
}

fn data_directory() -> io::Result<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".local/share")))
        .ok_or_else(|| io::Error::other(tr("не заданы XDG_DATA_HOME и HOME")))?;
    Ok(base.join("dimaster/terminfo"))
}

// выполняется на удалённой машине; ~/.terminfo ncurses находит сам
const REMOTE_INSTALL: &str = "mkdir -p ~/.terminfo && tic -x -o ~/.terminfo -";

/// `dimaster ssh-terminfo [опции ssh] хост`: один раз копирует запись на сервер,
/// после чего там можно работать с `term = "xterm-dimaster"`.
pub(crate) fn push_over_ssh(ssh_args: &[String]) -> io::Result<()> {
    let mut command = Command::new("ssh");
    command.args(ssh_args).arg(REMOTE_INSTALL);
    feed(command)?;
    println!(
        "{}",
        trf("Запись {name} установлена на сервере.", &[("name", NAME)])
    );
    Ok(())
}

// передаёт описание на вход команды и проверяет код возврата
fn feed(mut command: Command) -> io::Result<()> {
    let mut child = command.stdin(Stdio::piped()).spawn()?;
    if let Some(mut input) = child.stdin.take() {
        input.write_all(SOURCE.as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        return Err(io::Error::other(trf(
            "команда завершилась с {status}",
            &[("status", &status.to_string())],
        )));
    }
    Ok(())
}

/// Компилирует запись через `tic`; повторный вызов просто перезаписывает её.
fn install() -> io::Result<PathBuf> {
    let directory = data_directory()?;
    std::fs::create_dir_all(&directory)?;
    let mut tic = Command::new("tic");
    tic.args(["-x", "-o"]).arg(&directory).arg("-");
    feed(tic)?;
    Ok(directory)
}
