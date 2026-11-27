//! Процесс в PTY и потоки обмена с ним.
//!
//! Вывод разбирается в отдельном потоке прямо в общий терминал, а ввод пишется
//! из третьего: ни поток вывода, ни программа, переставшая читать ввод, не
//! замораживают окно.

use dimaster_i18n::t;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
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

// чужие значения от внешнего DimaSter не должны наследоваться до подстановки новых
fn reset_env(command: &mut Command, socket: Option<&Path>) {
    command
        .env_remove("DIMASTER_SOCKET")
        .env_remove(RESTORE_ENV)
        .env_remove("DIMASTER_STARTUP_FETCH")
        .env_remove("DIMASTER_FETCH_EARLY")
        .env_remove("DIMASTER_BIN")
        .env_remove("DIMASTER_COLOR_NANO")
        .env_remove("DIMASTER_COLOR_LS");
    if let Some(socket) = socket {
        command.env("DIMASTER_SOCKET", socket);
    }
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
    reset_env(&mut command, launch.socket);
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

/// Очередь к потоку записи и объём того, что в ней ждёт.
#[derive(Clone)]
struct Input {
    sender: Sender<Vec<u8>>,
    queued: Arc<AtomicUsize>,
}

impl Input {
    fn send(&self, bytes: Vec<u8>) -> bool {
        self.queued.fetch_add(bytes.len(), Ordering::Relaxed);
        self.sender.send(bytes).is_ok()
    }

    // программа, которая спрашивает, но не читает, не должна растить память
    fn reply(&self, bytes: Vec<u8>) -> bool {
        self.queued.load(Ordering::Relaxed) > dimaster_limits::REPLY_BACKLOG_BYTES.size()
            || self.send(bytes)
    }
}

/// Запущенный shell и потоки обмена с ним.
pub(crate) struct Session {
    pty: Pty,
    input: Input,
}

impl Session {
    /// Запускает shell и поток, который разбирает его вывод прямо в терминал.
    pub(crate) fn spawn(launch: &Launch<'_>, size: WindowSize, output: Output) -> io::Result<Self> {
        let pty = Pty::spawn(command(launch), size)?;
        let (sender, input_rx) = channel();
        let queued = Arc::default();
        let input = Input { sender, queued };
        let reader = pty.try_clone_master()?;
        let writer = pty.try_clone_master()?;
        let replies = input.clone();
        let queued = input.queued.clone();
        thread::spawn(move || read_loop(reader, &output, &replies));
        thread::spawn(move || write_loop(writer, &input_rx, &queued));
        Ok(Self { pty, input })
    }

    pub(crate) fn pid(&self) -> u32 {
        self.pty.id()
    }

    pub(crate) fn send(&self, bytes: Vec<u8>) {
        if !bytes.is_empty() && !self.input.send(bytes) {
            log::debug!("{}", t!("shell уже закрыт, ввод отброшен"));
        }
    }

    pub(crate) fn resize(&self, size: WindowSize) {
        if let Err(e) = self.pty.resize(size) {
            log::error!(
                "{}",
                t!("Не удалось изменить размер PTY: {error}", error = e)
            );
        }
    }
}

// блокировка берётся на одну порцию и отдаётся на время работы ffmpeg
fn feed(terminal: &SharedTerminal, mut bytes: &[u8]) -> Vec<u8> {
    while !bytes.is_empty() {
        let (used, slow) = terminal.lock().advance_part(bytes);
        bytes = &bytes[used..];
        if let Some(slow) = slow {
            let image = slow.decode();
            terminal.lock().finish_image(image);
        }
    }
    terminal.lock().take_reply()
}

fn read_loop(mut reader: impl Read, output: &Output, replies: &Input) {
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
        if !reply.is_empty() && !replies.reply(reply) {
            return;
        }
        if output.signals.raise() {
            (output.wake)();
        }
    }
}

fn write_loop(mut writer: impl Write, input: &Receiver<Vec<u8>>, queued: &AtomicUsize) {
    for bytes in input {
        if let Err(e) = writer.write_all(&bytes) {
            log::debug!("{}", t!("запись в PTY прекращена: {error}", error = e));
            return;
        }
        queued.fetch_sub(bytes.len(), Ordering::Relaxed);
    }
}
