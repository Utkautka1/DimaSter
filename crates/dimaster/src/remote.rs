//! Управление окнами из командной строки через Unix-сокет.
//!
//! Через сокет можно вводить команды в shell и читать экран, поэтому он лежит
//! в личном каталоге `$XDG_RUNTIME_DIR` с правами 0700. Запрос — аргументы
//! через NUL, ответ — строка `ok`/`err` и текст.

use dimaster_i18n::t;
use dimaster_i18n::tr;
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::thread;
use std::time::Duration;

const MAX_REQUEST: u64 = 1 << 20;
const TIMEOUT: Duration = Duration::from_secs(3);
const USAGE: &str = "dimaster ctl [list | info | get-text | send-text <текст> | action <имя> | set-title <текст> | reload | new-window]";

/// Запрос, который выполняет окно.
#[derive(Debug)]
pub(crate) struct Request {
    pub(crate) args: Vec<String>,
    pub(crate) reply: Sender<Result<String, String>>,
}

/// Открытый сокет; файл удаляется вместе с объектом.
#[derive(Debug)]
pub(crate) struct Server {
    pub(crate) path: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn directory() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?).join("dimaster"))
}

/// Сокеты окон, которые отвечают на соединение.
pub(crate) fn live_sockets() -> Vec<PathBuf> {
    let Some(directory) = directory() else {
        return Vec::new();
    };
    let entries = std::fs::read_dir(directory).into_iter().flatten().flatten();
    let mut live: Vec<_> = entries
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "sock"))
        .filter(|p| UnixStream::connect(p).is_ok())
        .collect();
    live.sort();
    live
}

fn bind() -> io::Result<(UnixListener, PathBuf)> {
    let directory = directory().ok_or_else(|| io::Error::other(tr("не задан XDG_RUNTIME_DIR")))?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&directory)?;
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
    let path = directory.join(format!("{}.sock", std::process::id()));
    // остаток от прежнего процесса с тем же номером
    let _ = std::fs::remove_file(&path);
    Ok((UnixListener::bind(&path)?, path))
}

fn parse(raw: &[u8]) -> Vec<String> {
    raw.split(|&b| b == 0)
        .map(|a| String::from_utf8_lossy(a).into_owned())
        .collect()
}

fn serve(mut stream: UnixStream, deliver: &impl Fn(Request)) -> io::Result<()> {
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    let mut raw = Vec::new();
    (&mut stream).take(MAX_REQUEST).read_to_end(&mut raw)?;
    if raw.is_empty() {
        return Ok(());
    }
    let (reply, answer) = channel();
    deliver(Request {
        args: parse(&raw),
        reply,
    });
    let result = answer
        .recv_timeout(TIMEOUT)
        .unwrap_or_else(|_| Err(tr("окно не ответило")));
    match result {
        Ok(text) => write!(stream, "ok\n{text}"),
        Err(text) => write!(stream, "err\n{text}"),
    }
}

/// `deliver` передаёт запрос в цикл событий окна.
pub(crate) fn listen(deliver: impl Fn(Request) + Send + 'static) -> Option<Server> {
    let (listener, path) = bind()
        .inspect_err(|e| log::warn!("{}", t!("Управление из CLI недоступно: {error}", error = e)))
        .ok()?;
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            if let Err(e) = serve(stream, &deliver) {
                log::debug!("{}", t!("сокет управления: {error}", error = e));
            }
        }
    });
    Some(Server { path })
}

fn call(socket: &Path, args: &[String]) -> io::Result<String> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(TIMEOUT * 2))?;
    stream.write_all(args.join("\0").as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut answer = String::new();
    stream.read_to_string(&mut answer)?;
    match answer.split_once('\n') {
        Some(("ok", text)) => Ok(text.to_owned()),
        Some((_, text)) => Err(io::Error::other(text.to_owned())),
        None => Err(io::Error::other(tr("пустой ответ"))),
    }
}

// своё окно, если команда запущена внутри DimaSter, иначе единственное живое
fn target() -> io::Result<PathBuf> {
    if let Some(own) = std::env::var_os("DIMASTER_SOCKET") {
        return Ok(PathBuf::from(own));
    }
    match live_sockets().as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(io::Error::other(tr("нет запущенных окон DimaSter"))),
        _ => Err(io::Error::other(tr(
            "окон несколько: задайте DIMASTER_SOCKET (см. ctl list)",
        ))),
    }
}

/// Клиентская часть: `dimaster ctl …`.
pub(crate) fn client(args: &[String]) -> io::Result<()> {
    let text = match args.first().map(String::as_str) {
        None => return Err(io::Error::other(tr(USAGE))),
        Some("list") => live_sockets()
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n"),
        Some(_) => call(&target()?, args)?,
    };
    if !text.is_empty() {
        println!("{text}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_through_socket() {
        let (client, server) = UnixStream::pair().unwrap();
        let worker = thread::spawn(move || {
            serve(server, &|r: Request| {
                r.reply.send(Ok(r.args.join("|"))).unwrap()
            })
        });
        let mut stream = client;
        stream.write_all(b"send-text\0a b").unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        let mut answer = String::new();
        stream.read_to_string(&mut answer).unwrap();
        worker.join().unwrap().unwrap();
        assert_eq!(answer, "ok\nsend-text|a b");
    }
}
