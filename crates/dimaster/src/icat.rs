//! `dimaster icat`: показ картинок, анимаций и видео прямо в терминале.
//!
//! Картинки уходят собственным протоколом одним куском — декодирует их сам
//! терминал, поэтому команда работает и по SSH. Видео разбирает `ffmpeg`, а
//! кадры передаются протоколом Kitty с заменой предыдущего.

use dimaster_config::Tuning;
use dimaster_i18n::tr;
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use dimaster_image::encode_base64;

const VIDEO: [&str; 6] = ["mp4", "mkv", "webm", "mov", "avi", "m4v"];
// кадр такой ширины успевает передаваться через PTY
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

// размер и частота кадров после масштабирования
#[derive(Clone, Copy)]
struct Frame {
    width: usize,
    height: usize,
    fps: u32,
}

// высота кадра после масштабирования до ширины `target`, чётная
fn video_height(path: &Path, target: usize) -> io::Result<usize> {
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
        (Some(width), Some(height)) if width > 0 => Ok((height * target / width).max(2) & !1),
        _ => Err(io::Error::other(tr("в файле не найдено видео"))),
    }
}

fn send_frame(out: &mut impl Write, pixels: &[u8], frame: Frame) -> io::Result<()> {
    let Frame { width, height, .. } = frame;
    let data = encode_base64(pixels);
    let chunks: Vec<&[u8]> = data.as_bytes().chunks(CHUNK).collect();
    write!(out, "\x1b[H")?;
    for (index, chunk) in chunks.iter().enumerate() {
        let more = u8::from(index + 1 < chunks.len());
        if index == 0 {
            let head = format!("a=T,f=32,s={width},v={height},i=1,p=1,C=1,q=2");
            write!(out, "\x1b_G{head},m={more};")?;
        } else {
            write!(out, "\x1b_Gm={more};")?;
        }
        out.write_all(chunk)?;
        write!(out, "\x1b\\")?;
    }
    out.flush()
}

fn stream(frames: &mut impl Read, frame: Frame, out: &mut impl Write) -> io::Result<()> {
    let mut pixels = vec![0; frame.width * frame.height * 4];
    let period = Duration::from_secs(1) / frame.fps;
    let mut due = Instant::now();
    while frames.read_exact(&mut pixels).is_ok() {
        send_frame(out, &pixels, frame)?;
        due += period;
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
    }
    Ok(())
}

fn decoder(path: &Path, filter: &str) -> io::Result<Child> {
    Command::new("ffmpeg")
        .args(["-v", "error", "-nostdin", "-i"])
        .arg(path)
        .args([
            "-vf", filter, "-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1",
        ])
        .stdout(Stdio::piped())
        .spawn()
}

// видео идёт на альтернативном экране, чтобы не засорять историю; без звука
fn play(path: &Path, tuning: &Tuning) -> io::Result<()> {
    let (width, fps) = (tuning.video_width, tuning.video_fps);
    let height = video_height(path, width)?;
    let frame = Frame { width, height, fps };
    let filter = format!("fps={fps},scale={width}:{height}");
    let mut ffmpeg = decoder(path, &filter)?;
    let mut frames = ffmpeg
        .stdout
        .take()
        .ok_or_else(|| io::Error::other(tr("нет вывода ffmpeg")))?;
    let mut out = io::stdout().lock();
    write!(out, "\x1b[?1049h\x1b[?25l")?;
    let result = stream(&mut frames, frame, &mut out);
    write!(out, "\x1b_Ga=d,d=I,i=1,q=2\x1b\\\x1b[?25h\x1b[?1049l")?;
    out.flush()?;
    ffmpeg.wait()?;
    result
}

fn one(path: &Path, tuning: &Tuning) -> io::Result<()> {
    if is_video(path) {
        return play(path, tuning);
    }
    let mut out = io::stdout().lock();
    show(&mut out, &std::fs::read(path)?, "")?;
    writeln!(out)
}

pub(crate) fn run(files: &[String]) -> io::Result<()> {
    let tuning = crate::settings::read_or_default().tuning;
    if std::env::var("TERM_PROGRAM").as_deref() != Ok("dimaster") {
        return Err(io::Error::other(tr("icat работает внутри DimaSter")));
    }
    files.iter().try_for_each(|file| {
        one(Path::new(file), &tuning).map_err(|e| io::Error::other(format!("{file}: {e}")))
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
