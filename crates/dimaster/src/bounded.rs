//! Запуск внешних программ с тайм-аутом: зависшие `git` или `docker` не
//! должны останавливать опрос и копить потоки.

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

// больше мегабайтов эти программы не печатают
const MAX_OUTPUT: u64 = 16 << 20;

/// Вывод успешно завершившейся программы; `None` — отказ или тайм-аут.
pub(crate) fn output(mut command: Command, limit: Duration) -> Option<String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let text = read_within(&mut child, limit);
    if text.is_none() {
        let _ = child.kill();
    }
    // дожидаемся всегда, чтобы не оставлять зомби
    let status = child.wait().ok()?;
    text.filter(|_| status.success())
}

// чтение в своём потоке: конец вывода означает, что программа закончила
fn read_within(child: &mut Child, limit: Duration) -> Option<String> {
    let stdout = child.stdout.take()?;
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let read = stdout.take(MAX_OUTPUT).read_to_end(&mut bytes);
        // получатель мог уже перестать ждать
        let _ = send.send(read.map(|_| bytes));
    });
    let bytes = receive.recv_timeout(limit).ok()?.ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn hung_program_is_killed() {
        let mut quick = Command::new("echo");
        quick.arg("ok");
        assert_eq!(
            output(quick, Duration::from_secs(5)).as_deref(),
            Some("ok\n")
        );
        let mut hung = Command::new("sleep");
        hung.arg("30");
        let started = Instant::now();
        assert!(output(hung, Duration::from_millis(100)).is_none());
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
