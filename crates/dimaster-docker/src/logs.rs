//! Живые логи: `docker logs --follow` в фоне и ограниченный буфер строк.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;

use dimaster_vt::{Parser, Perform};

#[derive(Default)]
struct Plain(String);

impl Perform for Plain {
    fn print(&mut self, ch: char) {
        self.0.push(ch);
    }

    fn execute(&mut self, byte: u8) {
        if byte == b'\t' {
            self.0.push_str("    ");
        }
    }
}

/// Текст без цветов и прочих управляющих последовательностей.
pub(crate) fn plain(bytes: &[u8]) -> String {
    let mut text = Plain::default();
    Parser::default().advance(&mut text, bytes);
    text.0
}

fn forward(stream: impl Read + Send + 'static, send: SyncSender<String>) {
    thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut line = Vec::new();
        while (&mut reader)
            .take(dimaster_limits::DOCKER_LOG_LINE_BYTES.get())
            .read_until(b'\n', &mut line)
            .unwrap_or(0)
            > 0
        {
            if send.send(plain(&line)).is_err() {
                return;
            }
            line.clear();
        }
    });
}

/// Идущий вывод программы; она убивается вместе с объектом.
pub(crate) struct Tail {
    child: Child,
    incoming: Receiver<String>,
    pub(crate) lines: VecDeque<String>,
}

impl Tail {
    pub(crate) fn start(mut command: Command) -> std::io::Result<Self> {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let (send, incoming) = sync_channel(dimaster_limits::DOCKER_LOG_QUEUE.size());
        if let Some(out) = child.stdout.take() {
            forward(out, send.clone());
        }
        if let Some(errors) = child.stderr.take() {
            forward(errors, send);
        }
        Ok(Self {
            child,
            incoming,
            lines: VecDeque::new(),
        })
    }

    /// Забирает пришедшее; `true` — появились новые строки.
    pub(crate) fn poll(&mut self) -> bool {
        let mut fresh = false;
        for line in self
            .incoming
            .try_iter()
            .take(dimaster_limits::DOCKER_LOG_QUEUE.size())
        {
            if self.lines.len() == dimaster_limits::DOCKER_LOG_LINES.size() {
                self.lines.pop_front();
            }
            self.lines.push_back(line);
            fresh = true;
        }
        fresh
    }
}

impl Drop for Tail {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_are_removed() {
        assert_eq!(plain(b"\x1b[32mok\x1b[0m\tdone\r\n"), "ok    done");
    }
}
