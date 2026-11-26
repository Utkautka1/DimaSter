//! Процесс в PTY и потоки обмена с ним.
//!
//! Чтение и запись вынесены из UI-потока: иначе большая вставка в программу,
//! которая не читает ввод, заморозила бы окно.

use std::io::{self, Read, Write};
use std::process::Command;
use std::sync::mpsc::{Receiver, Sender, SyncSender, channel, sync_channel};
use std::thread;

use dimaster_pty::{Pty, WindowSize};

// столько порций может ждать отрисовки; дальше чтение тормозит программу
const OUTPUT_QUEUE: usize = 64;

/// Команда с окружением терминала.
pub(crate) fn command(args: &[String]) -> Command {
    let mut command = Command::new(&args[0]);
    let (term, terminfo) = crate::terminfo::select();
    if let Some(directory) = terminfo {
        command.env("TERMINFO", directory);
    }
    command
        .args(&args[1..])
        .env("TERM", term)
        .env("COLORTERM", "truecolor")
        .env("TERM_PROGRAM", "dimaster");
    command
}

/// Запущенный shell и каналы к нему.
pub(crate) struct Session {
    pty: Pty,
    input: Sender<Vec<u8>>,
    pub(crate) output: Receiver<Vec<u8>>,
}

impl Session {
    /// `wake` будит цикл событий; пустой канал после пробуждения — не ошибка.
    pub(crate) fn spawn(
        args: &[String],
        size: WindowSize,
        wake: impl Fn() + Send + 'static,
    ) -> io::Result<Self> {
        let pty = Pty::spawn(command(args), size)?;
        let (output_tx, output) = sync_channel(OUTPUT_QUEUE);
        let (input, input_rx) = channel();
        let reader = pty.try_clone_master()?;
        let writer = pty.try_clone_master()?;
        thread::spawn(move || read_loop(reader, &output_tx, wake));
        thread::spawn(move || write_loop(writer, &input_rx));
        Ok(Self { pty, input, output })
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

// Конец потока обозначается пустой порцией
fn read_loop(mut reader: impl Read, output: &SyncSender<Vec<u8>>, wake: impl Fn()) {
    let mut buf = [0; 64 * 1024];
    loop {
        // EIO после выхода shell — обычное завершение
        let read = reader.read(&mut buf).unwrap_or(0);
        if output.send(buf[..read].to_vec()).is_err() {
            return;
        }
        wake();
        if read == 0 {
            return;
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
