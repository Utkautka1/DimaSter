//! Процесс в PTY и потоки обмена с ним.
//!
//! Вывод разбирается в отдельном потоке прямо в общий терминал, а ввод пишется
//! из третьего: ни поток вывода, ни программа, переставшая читать ввод, не
//! замораживают окно.

use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use dimaster_pty::{Pty, WindowSize};

use crate::integration;
use crate::restore::RESTORE_ENV;
use crate::shared::{SharedTerminal, Signals};

/// Что и как запускать в PTY.
#[derive(Debug)]
pub(crate) struct Launch<'a> {
    pub(crate) args: &'a [String],
    pub(crate) term_name: &'a str,
    pub(crate) directory: Option<&'a Path>,
    // сокет управления этим окном
    pub(crate) socket: Option<&'a Path>,
    /// Подключать ли интеграцию с shell.
    pub(crate) integrate: bool,
    /// Переменные из настроек.
    pub(crate) env: &'a [(String, String)],
}

/// Команда с окружением терминала.
pub(crate) fn command(launch: &Launch<'_>) -> Command {
    let mut command = Command::new(&launch.args[0]);
    let (term, terminfo) = crate::terminfo::select(launch.term_name);
    if let Some(directory) = terminfo {
        command.env("TERMINFO", directory);
    }
    if let Some(directory) = launch.directory {
        command.current_dir(directory);
    }
    // чужие значения от внешнего DimaSter не должны наследоваться
    command
        .env_remove("DIMASTER_SOCKET")
        .env_remove(RESTORE_ENV);
    if let Some(socket) = launch.socket {
        command.env("DIMASTER_SOCKET", socket);
    }
    command.envs(launch.env.iter().cloned());
    if launch.integrate {
        let injection = integration::inject(launch.args);
        command.args(injection.args).envs(injection.env);
    }
    command
        .args(&launch.args[1..])
        .env("TERM", term)
        .env("COLORTERM", "truecolor")
        .env("TERM_PROGRAM", "dimaster");
    command
}

/// Куда попадает вывод shell и как об этом узнаёт окно.
pub(crate) struct Output {
    pub(crate) terminal: SharedTerminal,
    pub(crate) signals: Arc<Signals>,
    pub(crate) wake: Box<dyn Fn() + Send>,
}

/// Запущенный shell и потоки обмена с ним.
pub(crate) struct Session {
    pty: Pty,
    input: Sender<Vec<u8>>,
}

impl Session {
    /// Запускает shell и поток, который разбирает его вывод прямо в терминал.
    pub(crate) fn spawn(launch: &Launch<'_>, size: WindowSize, output: Output) -> io::Result<Self> {
        let pty = Pty::spawn(command(launch), size)?;
        let (input, input_rx) = channel();
        let reader = pty.try_clone_master()?;
        let writer = pty.try_clone_master()?;
        let replies = input.clone();
        thread::spawn(move || read_loop(reader, &output, &replies));
        thread::spawn(move || write_loop(writer, &input_rx));
        Ok(Self { pty, input })
    }

    pub(crate) fn pid(&self) -> u32 {
        self.pty.id()
    }

    pub(crate) fn send(&self, bytes: Vec<u8>) {
        if !bytes.is_empty() && self.input.send(bytes).is_err() {
            log::debug!("shell уже закрыт, ввод отброшен");
        }
    }

    pub(crate) fn resize(&self, size: WindowSize) {
        if let Err(e) = self.pty.resize(size) {
            log::error!("Не удалось изменить размер PTY: {e}");
        }
    }
}

// блокировка берётся на одну порцию и сразу отдаётся
fn feed(terminal: &SharedTerminal, bytes: &[u8]) -> Vec<u8> {
    let mut terminal = terminal.lock();
    terminal.advance(bytes);
    terminal.take_reply()
}

fn read_loop(mut reader: impl Read, output: &Output, replies: &Sender<Vec<u8>>) {
    let mut buf = [0; 64 * 1024];
    loop {
        // EIO после выхода shell — обычное завершение
        let read = reader.read(&mut buf).unwrap_or(0);
        if read == 0 {
            output.signals.close();
            output.signals.raise();
            return (output.wake)();
        }
        let reply = feed(&output.terminal, &buf[..read]);
        if !reply.is_empty() && replies.send(reply).is_err() {
            return;
        }
        if output.signals.raise() {
            (output.wake)();
        }
    }
}

fn write_loop(mut writer: impl Write, input: &Receiver<Vec<u8>>) {
    for bytes in input {
        if let Err(e) = writer.write_all(&bytes) {
            log::debug!("запись в PTY прекращена: {e}");
            return;
        }
    }
}
