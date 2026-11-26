//! Форматы без своего декодера: первый кадр через `ffmpeg`.
//!
//! Данные приходят от программ, поэтому ffmpeg разрешён только протокол
//! `pipe`: подсунутый плейлист не сможет прочитать файлы или открыть сеть.

use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;

use crate::{Animation, decode_png};

pub(crate) fn decode(data: &[u8]) -> Option<Animation> {
    let mut child = Command::new("ffmpeg")
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
        .ok()?;
    let mut input = child.stdin.take()?;
    let owned = data.to_vec();
    // пишем из другого потока: иначе оба процесса могут встать на полных каналах
    let writer = thread::spawn(move || input.write_all(&owned));
    let output = child.wait_with_output().ok()?;
    let _ = writer.join();
    let image = decode_png(&output.stdout)?;
    Some(Animation::still(image.width, image.height, image.pixels))
}
