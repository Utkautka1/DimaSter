//! Удалённое управление: открытый просмотрщик слушает сокет, а
//! `dimaster view --send "команда"` выполняет в нём команду.
//!
//! Включается только явно. Сокет лежит в `XDG_RUNTIME_DIR`, закрытом для
//! других пользователей; без этого каталога управления нет.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

const PREFIX: &str = "dimaster-view-";
// длиннее команды не бывают
const LIMIT: u64 = 64 * 1024;

/// Присланная команда и куда ответить.
pub(crate) struct Request {
    pub(crate) line: String,
    reply: UnixStream,
}

impl Request {
    pub(crate) fn answer(mut self, text: &str) {
        // отправитель мог уже уйти — это не ошибка просмотрщика
        let _ = writeln!(self.reply, "{text}");
    }
}

/// Слушающий сокет; файл удаляется вместе с ним.
pub(crate) struct Server {
    path: PathBuf,
    pub(crate) requests: Receiver<Request>,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn runtime() -> io::Result<PathBuf> {
    let directory = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    directory.ok_or_else(|| io::Error::other("XDG_RUNTIME_DIR"))
}

fn receive(stream: UnixStream, requests: &Sender<Request>) -> io::Result<()> {
    // молчащий клиент не должен останавливать остальных
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut line = String::new();
    BufReader::new((&stream).take(LIMIT)).read_line(&mut line)?;
    let request = Request {
        line: line.trim_end().to_owned(),
        reply: stream,
    };
    requests.send(request).map_err(io::Error::other)
}

fn listen_at(path: PathBuf) -> io::Result<Server> {
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    let (sender, requests) = channel();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let _ = receive(stream, &sender);
        }
    });
    Ok(Server { path, requests })
}

pub(crate) fn listen() -> io::Result<Server> {
    let name = format!("{PREFIX}{}.sock", std::process::id());
    listen_at(runtime()?.join(name))
}

// сокеты в каталоге, новые первыми
fn sockets(directory: &Path) -> io::Result<Vec<PathBuf>> {
    let ours = |path: &PathBuf| {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        name.starts_with(PREFIX) && name.ends_with(".sock")
    };
    let entries = std::fs::read_dir(directory)?.flatten().map(|e| e.path());
    let mut found: Vec<_> = entries.filter(ours).collect();
    found.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
    found.reverse();
    Ok(found)
}

fn send_to(directory: &Path, line: &str) -> io::Result<String> {
    // от упавшего просмотрщика остаётся файл, к нему не подключиться
    let mut stream = sockets(directory)?
        .iter()
        .find_map(|p| UnixStream::connect(p).ok())
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    writeln!(stream, "{line}")?;
    let mut answer = String::new();
    stream.read_to_string(&mut answer)?;
    Ok(answer.trim_end().to_owned())
}

/// Выполняет команду в последнем открытом просмотрщике.
pub fn send(line: &str) -> io::Result<String> {
    send_to(&runtime()?, line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_goes_there_and_answer_back() {
        let directory =
            std::env::temp_dir().join(format!("dimaster-remote-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("каталог");
        let server = listen_at(directory.join(format!("{PREFIX}1.sock"))).expect("сокет");
        let from = directory.clone();
        let client = std::thread::spawn(move || send_to(&from, "o age"));
        let request = server.requests.recv().expect("команда");
        assert_eq!(request.line, "o age");
        request.answer("ok");
        assert_eq!(client.join().expect("клиент").expect("ответ"), "ok");
        drop(server);
        assert!(send_to(&directory, "x").is_err());
        std::fs::remove_dir_all(&directory).expect("уборка");
    }
}
