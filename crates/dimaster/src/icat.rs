//! `dimaster icat`: показ картинок, анимаций и видео прямо в терминале.
//!
//! Картинки уходят собственным протоколом одним куском — декодирует их сам
//! терминал, поэтому команда работает и по SSH. Видео разбирает `ffmpeg`, а
//! кадры передаются протоколом Kitty с заменой предыдущего.

use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use dimaster_image::encode_base64;

const VIDEO: [&str; 6] = ["mp4", "mkv", "webm", "mov", "avi", "m4v"];
// кадр такой ширины успевает передаваться через PTY
const VIDEO_WIDTH: usize = 480;
const VIDEO_FPS: u32 = 15;
// стандартный размер куска протокола Kitty
const CHUNK: usize = 4096;

/// Файл целиком собственным протоколом; `options` — например `c=20,r=10`.
pub(crate) fn show(out: &mut impl Write, data: &[u8], options: &str) -> io::Result<()> {
    write!(out, "\x1b_Da=show,{options};{}\x1b\\", encode_base64(data))
}

fn is_video(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    VIDEO.contains(&extension.to_ascii_lowercase().as_str())
}

// высота кадра после масштабирования до VIDEO_WIDTH, чётная
fn video_height(path: &Path) -> io::Result<usize> {
    let output = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0"])
        .args(["-show_entries", "stream=width,height", "-of", "csv=p=0"])
        .arg(path)
        .output()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let mut numbers = text
        .trim()
        .split(',')
        .filter_map(|n| n.parse::<usize>().ok());
    match (numbers.next(), numbers.next()) {
        (Some(width), Some(height)) if width > 0 => Ok((height * VIDEO_WIDTH / width).max(2) & !1),
        _ => Err(io::Error::other("в файле не найдено видео")),
    }
}

fn send_frame(out: &mut impl Write, pixels: &[u8], height: usize) -> io::Result<()> {
    let data = encode_base64(pixels);
    let chunks: Vec<&[u8]> = data.as_bytes().chunks(CHUNK).collect();
    write!(out, "\x1b[H")?;
    for (index, chunk) in chunks.iter().enumerate() {
        let more = u8::from(index + 1 < chunks.len());
        if index == 0 {
            let head = format!("a=T,f=32,s={VIDEO_WIDTH},v={height},i=1,p=1,C=1,q=2");
            write!(out, "\x1b_G{head},m={more};")?;
        } else {
            write!(out, "\x1b_Gm={more};")?;
        }
        out.write_all(chunk)?;
        write!(out, "\x1b\\")?;
    }
    out.flush()
}

fn stream(frames: &mut impl Read, height: usize, out: &mut impl Write) -> io::Result<()> {
    let mut pixels = vec![0; VIDEO_WIDTH * height * 4];
    let period = Duration::from_secs(1) / VIDEO_FPS;
    let mut due = Instant::now();
    while frames.read_exact(&mut pixels).is_ok() {
        send_frame(out, &pixels, height)?;
        due += period;
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
    }
    Ok(())
}

// видео идёт на альтернативном экране, чтобы не засорять историю; без звука
fn play(path: &Path) -> io::Result<()> {
    let height = video_height(path)?;
    let filter = format!("fps={VIDEO_FPS},scale={VIDEO_WIDTH}:{height}");
    let mut ffmpeg = Command::new("ffmpeg")
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(path)
        .args([
            "-vf", &filter, "-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1",
        ])
        .stdout(Stdio::piped())
        .spawn()?;
    let mut frames = ffmpeg
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("нет вывода ffmpeg"))?;
    let mut out = io::stdout().lock();
    write!(out, "\x1b[?1049h\x1b[?25l")?;
    let result = stream(&mut frames, height, &mut out);
    write!(out, "\x1b_Ga=d,d=I,i=1,q=2\x1b\\\x1b[?25h\x1b[?1049l")?;
    out.flush()?;
    ffmpeg.wait()?;
    result
}

fn one(path: &Path) -> io::Result<()> {
    if is_video(path) {
        return play(path);
    }
    let mut out = io::stdout().lock();
    show(&mut out, &std::fs::read(path)?, "")?;
    writeln!(out)
}

pub(crate) fn run(files: &[String]) -> io::Result<()> {
    if std::env::var("TERM_PROGRAM").as_deref() != Ok("dimaster") {
        return Err(io::Error::other("icat работает внутри DimaSter"));
    }
    files.iter().try_for_each(|file| {
        one(Path::new(file)).map_err(|e| io::Error::other(format!("{file}: {e}")))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_is_detected_by_extension() {
        assert!(is_video(Path::new("a.MP4")));
        assert!(!is_video(Path::new("a.gif")));
    }
}
