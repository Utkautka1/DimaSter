//! Пределы запроса: рекурсивный запрос без `LIMIT` иначе занял бы всю память
//! или никогда не закончился.

use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rusqlite::Connection;

use crate::Value;

/// Сколько запросу можно работать и занимать.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub time: Duration,
    /// Примерный объём результата в байтах.
    pub bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            time: Duration::from_secs(60),
            bytes: 1 << 30,
        }
    }
}

pub(crate) fn row_bytes(row: &[Value]) -> usize {
    let text = |value: &Value| match value {
        Value::Text(text) => text.len(),
        _ => 0,
    };
    row.iter().map(|v| size_of::<Value>() + text(v)).sum()
}

/// Прерывает запрос по истечении времени; снимается при удалении.
pub(crate) struct Watchdog {
    done: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Watchdog {
    pub(crate) fn start(connection: &Connection, time: Duration) -> Self {
        let handle = connection.get_interrupt_handle();
        let (done, finished) = channel();
        let thread = thread::spawn(move || {
            if finished.recv_timeout(time) == Err(RecvTimeoutError::Timeout) {
                handle.interrupt();
            }
        });
        Self {
            done: Some(done),
            thread: Some(thread),
        }
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        drop(self.done.take());
        // дожидаемся, чтобы запоздалое прерывание не задело следующий запрос
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
