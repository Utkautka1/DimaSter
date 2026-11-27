//! Запуск внешних программ с тайм-аутом: зависшая программа убивается.

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::Duration;

// больше мегабайтов эти программы не печатают
const MAX_OUTPUT: u64 = 16 << 20;

// чтение в своём потоке, чтобы программа не встала на полном канале
fn reading(stream: Option<impl Read + Send + 'static>) -> Receiver<String> {
    let (send, receive) = channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(stream) = stream {
            // оборванное чтение оставляет то, что успели получить
            let _ = stream.take(MAX_OUTPUT).read_to_end(&mut bytes);
        }
        let _ = send.send(String::from_utf8_lossy(&bytes).into_owned());
    });
    receive
}

fn finish(child: &mut Child, limit: Duration) -> Result<String, String> {
    let output = reading(child.stdout.take());
    let errors = reading(child.stderr.take());
    let Ok(text) = output.recv_timeout(limit) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err("timeout".to_owned());
    };
    // дожидаемся всегда, чтобы не оставлять зомби
    let status = child.wait().map_err(|e| e.to_string())?;
    match status.success() {
        true => Ok(text),
        false => Err(errors
            .recv_timeout(limit)
            .unwrap_or_default()
            .trim()
            .to_owned()),
    }
}

/// Вывод успешной программы, иначе текст её ошибки.
pub fn capture(mut command: Command, limit: Duration) -> Result<String, String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    finish(&mut child, limit)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    fn sh(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        command
    }

    #[test]
    fn output_errors_and_hung_programs() {
        let long = Duration::from_secs(5);
        assert_eq!(capture(sh("echo ok"), long).as_deref(), Ok("ok\n"));
        assert_eq!(capture(sh("echo bad >&2; exit 1"), long), Err("bad".into()));
        let started = Instant::now();
        let hung = capture(sh("exec sleep 30"), Duration::from_millis(100));
        assert!(hung.is_err() && started.elapsed() < long);
    }
}
