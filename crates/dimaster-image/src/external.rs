//! Форматы без своего декодера: первый кадр через `ffmpeg`.
//!
//! Данные приходят от программ, поэтому ffmpeg разрешён только протокол
//! `pipe`: подсунутый плейлист не сможет прочитать файлы или открыть сеть.

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Instant;

use crate::{Animation, decode_png};

pub(crate) fn decode(data: &[u8]) -> Option<Animation> {
    let mut child = spawn_ffmpeg()?;
    let mut input = child.stdin.take()?;
    let mut output = child.stdout.take()?;
    let owned = data.to_vec();
    // пишем и читаем из других потоков: иначе оба процесса встанут на каналах
    let writer = thread::spawn(move || input.write_all(&owned));
    let reader = thread::spawn(move || {
        let mut png = Vec::new();
        output
            .by_ref()
            .take(dimaster_limits::DECODER_OUTPUT_BYTES.get())
            .read_to_end(&mut png)
            .map(|_| png)
    });
    wait_or_kill(&mut child);
    let _ = writer.join();
    let image = decode_png(&reader.join().ok()?.ok()?)?;
    Some(Animation::still(image.width, image.height, image.pixels))
}

fn spawn_ffmpeg() -> Option<Child> {
    Command::new("ffmpeg")
        .args(["-v", "error", "-protocol_whitelist", "pipe", "-i", "pipe:0"])
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale='min(4096,iw)':-1",
            "-pix_fmt",
            "rgba",
        ])
        .args(["-f", "image2pipe", "-c:v", "png", "pipe:1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()
}

// вызывающий поток ждёт результата: подвисший ffmpeg нельзя ждать бесконечно
fn wait_or_kill(child: &mut Child) {
    let deadline = Instant::now() + dimaster_limits::DECODER_TIMEOUT.secs();
    while matches!(child.try_wait(), Ok(None)) {
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            return;
        }
        thread::sleep(dimaster_limits::DECODER_POLL_MS.millis());
    }
}
