//! Графический протокол Kitty (APC `G`): вывод, анимация (`a=f`/`a=a`), `t=s` и `U=1`.

use std::collections::HashMap;
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use dimaster_image::{Animation, Frame, decode, decode_base64, inflate_zlib};

use super::{MAX_IMAGE_ROWS, Picture, VirtualPlacement};
use crate::terminal::State;

// предел на один файл или передачу по частям
const MAX_PAYLOAD: usize = 64 * 1024 * 1024;
const MAX_SIDE: usize = 8192;

/// Ключи команды и накопленные base64-данные.
#[derive(Debug, Default)]
pub(crate) struct Pending {
    keys: HashMap<u8, String>,
    data: Vec<u8>,
}

impl Pending {
    fn number(&self, key: u8) -> u32 {
        self.keys
            .get(&key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }

    fn signed(&self, key: u8) -> Option<i32> {
        self.keys.get(&key).and_then(|v| v.parse().ok())
    }

    fn letter(&self, key: u8, default: u8) -> u8 {
        self.keys
            .get(&key)
            .and_then(|v| v.bytes().next())
            .unwrap_or(default)
    }
}

fn parse(data: &[u8]) -> Pending {
    let split = data.iter().position(|&b| b == b';').unwrap_or(data.len());
    let header = String::from_utf8_lossy(&data[..split]);
    let keys = header
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .filter_map(|(k, v)| Some((k.bytes().next()?, v.to_owned())))
        .collect();
    Pending {
        keys,
        data: data.get(split + 1..).unwrap_or_default().to_vec(),
    }
}

// удалять можно только то, что создано для протокола во временном каталоге;
// ссылки разрешаются, чтобы путь не вывел наружу
fn is_temporary(path: &Path) -> bool {
    let marked = path
        .file_name()
        .is_some_and(|n| n.to_string_lossy().contains("tty-graphics-protocol"));
    let regular = std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file());
    let Ok(real) = path.canonicalize() else {
        return false;
    };
    let roots = [std::env::temp_dir(), "/tmp".into(), "/dev/shm".into()];
    let inside = roots
        .iter()
        .filter_map(|r| r.canonicalize().ok())
        .any(|r| real.starts_with(r));
    marked && regular && inside
}

// файлы ядра и устройства: даже открытие может иметь побочный эффект, а ответ
// терминала выдаёт удалённой программе, существует ли файл
fn is_readable_location(path: &Path) -> bool {
    let Ok(real) = path.canonicalize() else {
        return false;
    };
    let system = ["/proc", "/sys", "/dev"]
        .iter()
        .any(|root| real.starts_with(root));
    !system || real.starts_with("/dev/shm")
}

// путь проверяется до открытия и ещё раз по открытому дескриптору: между ними
// его могли подменить ссылкой
fn open_checked(path: &Path) -> Option<File> {
    let regular = std::fs::metadata(path).is_ok_and(|m| m.is_file());
    if !regular || !is_readable_location(path) {
        return None;
    }
    let file = File::open(path).ok()?;
    let opened = format!("/proc/self/fd/{}", file.as_raw_fd());
    let real = std::fs::read_link(opened).ok()?;
    let meta = file.metadata().ok()?;
    let fits = meta.is_file() && meta.len() <= MAX_PAYLOAD as u64;
    (fits && is_readable_location(&real)).then_some(file)
}

// программа вне нашей машины может прислать путь, поэтому только обычные файлы
fn read_file(path: &[u8], temporary: bool) -> Option<Vec<u8>> {
    let path = Path::new(std::str::from_utf8(path).ok()?);
    let mut data = Vec::new();
    open_checked(path)?
        .take(MAX_PAYLOAD as u64)
        .read_to_end(&mut data)
        .ok()?;
    if temporary && is_temporary(path) {
        let _ = std::fs::remove_file(path);
    }
    Some(data)
}

#[cfg(unix)]
fn read_shm(name: &[u8]) -> Option<Vec<u8>> {
    let raw = std::str::from_utf8(name).ok()?;
    let clean = raw.strip_prefix('/').unwrap_or(raw);
    if clean.is_empty() || clean.contains('/') || clean.contains('\0') {
        return None;
    }
    let c_name = CString::new(format!("/{clean}")).ok()?;
    // SAFETY: `c_name` — валидная C-строка без внутренних NUL-байтов;
    // дескриптор сразу оборачивается в `File`, а имя удаляется через `shm_unlink`.
    let fd = unsafe { libc::shm_open(c_name.as_ptr(), libc::O_RDONLY, 0) };
    if fd < 0 {
        return None;
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let data = read_bounded(file);
    unsafe { libc::shm_unlink(c_name.as_ptr()) };
    data
}

#[cfg(not(unix))]
fn read_shm(_: &[u8]) -> Option<Vec<u8>> {
    None
}

fn read_bounded(file: File) -> Option<Vec<u8>> {
    let meta = file.metadata().ok()?;
    if !meta.is_file() || meta.len() > MAX_PAYLOAD as u64 {
        return None;
    }
    let mut data = Vec::new();
    file.take(MAX_PAYLOAD as u64).read_to_end(&mut data).ok()?;
    Some(data)
}

fn raw_pixels(command: &Pending, data: Vec<u8>, depth: usize) -> Option<Animation> {
    let (width, height) = (command.number(b's') as usize, command.number(b'v') as usize);
    let sizes = (1..=MAX_SIDE).contains(&width) && (1..=MAX_SIDE).contains(&height);
    if !sizes || data.len() < width * height * depth {
        return None;
    }
    let pixels = data[..width * height * depth]
        .chunks(depth)
        .flat_map(|p| [p[0], p[1], p[2], p.get(3).copied().unwrap_or(255)])
        .collect();
    Some(Animation::still(width, height, pixels))
}

fn load(command: &Pending) -> Option<Animation> {
    let mut data = decode_base64(&command.data)?;
    match command.letter(b't', b'd') {
        b'd' => {}
        medium @ (b'f' | b't') => data = read_file(&data, medium == b't')?,
        b's' => data = read_shm(&data)?,
        _ => return None,
    }
    if command.letter(b'o', 0) == b'z' {
        data = inflate_zlib(&data, MAX_PAYLOAD * 4)?;
    }
    match command.number(b'f') {
        100 => decode(&data),
        24 => raw_pixels(command, data, 3),
        0 | 32 => raw_pixels(command, data, 4),
        _ => None,
    }
}

fn crop_rect(command: &Pending, picture: &Picture) -> Option<[u32; 4]> {
    let (x, y) = (command.number(b'x'), command.number(b'y'));
    let (w, h) = (command.number(b'w'), command.number(b'h'));
    if x == 0 && y == 0 && w == 0 && h == 0 {
        return None;
    }
    let (pw, ph) = picture.dimensions();
    let (pw, ph) = (pw as u32, ph as u32);
    if x >= pw || y >= ph {
        return None;
    }
    let cw = if w == 0 { pw - x } else { w.min(pw - x) };
    let ch = if h == 0 { ph - y } else { h.min(ph - y) };
    (cw > 0 && ch > 0).then_some([x, y, cw, ch])
}

fn blit_frame(
    canvas: &mut [u8],
    cw: usize,
    ch: usize,
    src: &Animation,
    at: (usize, usize),
    replace: bool,
) {
    let pixels = &src.frames[0].pixels;
    for row in 0..src.height.min(ch.saturating_sub(at.1)) {
        for col in 0..src.width.min(cw.saturating_sub(at.0)) {
            let si = (row * src.width + col) * 4;
            let di = ((at.1 + row) * cw + at.0 + col) * 4;
            if replace {
                canvas[di..di + 4].copy_from_slice(&pixels[si..si + 4]);
            } else {
                blend_pixel(&mut canvas[di..di + 4], &pixels[si..si + 4]);
            }
        }
    }
}

fn blend_pixel(dst: &mut [u8], src: &[u8]) {
    let a = u16::from(src[3]);
    let inv = 255 - a;
    for i in 0..3 {
        dst[i] = ((u16::from(src[i]) * a + u16::from(dst[i]) * inv) / 255) as u8;
    }
    dst[3] = dst[3].saturating_add(((a * u16::from(255 - dst[3])) / 255) as u8);
}

fn frame_delay(command: &Pending) -> u32 {
    match command.signed(b'z') {
        Some(z) if z < 0 => 1,
        Some(z) if z > 0 => z as u32,
        _ => 40,
    }
}

impl State {
    pub(crate) fn kitty(&mut self, data: &[u8]) {
        let part = parse(data);
        let more = part.number(b'm') == 1;
        // у продолжений есть только m, ключи берутся из первой части
        let mut command = match self.graphics.pending.take() {
            Some(mut first) => {
                first.data.extend_from_slice(&part.data);
                first
            }
            None => part,
        };
        if command.data.len() > MAX_PAYLOAD {
            return;
        }
        if more {
            self.graphics.pending = Some(command);
            return;
        }
        command.keys.remove(&b'm');
        self.kitty_command(&command);
    }

    fn kitty_command(&mut self, command: &Pending) {
        let action = command.letter(b'a', b't');
        let ok = match action {
            b't' | b'T' | b'q' => self.kitty_transmit(command, action),
            b'p' => self.kitty_put(command),
            b'f' => self.kitty_frame(command),
            b'a' => self.kitty_animate(command),
            b'd' => return self.kitty_delete(command),
            _ => false,
        };
        self.kitty_reply(command, ok);
    }

    fn kitty_transmit(&mut self, command: &Pending, action: u8) -> bool {
        let Some(animation) = load(command) else {
            return false;
        };
        if action == b'q' {
            return true;
        }
        let picture = self.graphics.picture(animation);
        let id = command.number(b'i');
        if id != 0 {
            self.graphics.keep(id, picture.clone());
        }
        if action == b'T' {
            self.kitty_show(command, picture);
        }
        true
    }

    fn kitty_put(&mut self, command: &Pending) -> bool {
        let Some(picture) = self.graphics.get(command.number(b'i')) else {
            return false;
        };
        self.kitty_show(command, picture);
        true
    }

    fn kitty_show(&mut self, command: &Pending, picture: Arc<Picture>) {
        let id = (command.number(b'i'), command.number(b'p'));
        let crop = crop_rect(command, &picture);
        let size = |key| Some(command.number(key) as usize);
        if command.number(b'U') == 1 {
            let (pw, ph) = picture.dimensions();
            let (cols, rows) = self.raw_image_cells(pw.max(1), ph.max(1), size(b'c'), size(b'r'));
            let placement = VirtualPlacement {
                picture,
                cols: cols.clamp(1, MAX_IMAGE_ROWS),
                rows: rows.clamp(1, MAX_IMAGE_ROWS),
            };
            self.graphics.put_virtual(id, placement);
            return;
        }
        let cells = self.image_cells(&picture, size(b'c'), size(b'r'), crop);
        if command.number(b'C') == 1 {
            self.place(picture, cells, id, crop);
        } else {
            self.place_and_advance(picture, cells, id, crop);
        }
    }

    fn kitty_frame(&mut self, command: &Pending) -> bool {
        let Some(picture) = self.graphics.get(command.number(b'i')) else {
            return false;
        };
        let Some(src) = load(command) else {
            return false;
        };
        let delay = frame_delay(command);
        let mut anim = picture.animation.write();
        let (cw, ch) = (anim.width, anim.height);
        let base = command
            .number(b'c')
            .checked_sub(1)
            .and_then(|i| anim.frames.get(i as usize));
        let mut pixels = base.map_or_else(|| vec![0; cw * ch * 4], |f| f.pixels.clone());
        let at = (command.number(b'x') as usize, command.number(b'y') as usize);
        blit_frame(&mut pixels, cw, ch, &src, at, command.number(b'X') == 1);
        let frame = Frame { pixels, delay };
        match command.number(b'r').checked_sub(1) {
            Some(idx) if (idx as usize) < anim.frames.len() => anim.frames[idx as usize] = frame,
            _ => anim.frames.push(frame),
        }
        picture.generation.fetch_add(1, Ordering::Relaxed);
        true
    }

    fn kitty_animate(&mut self, command: &Pending) -> bool {
        let Some(picture) = self.graphics.get(command.number(b'i')) else {
            return false;
        };
        self.apply_animation_state(command, &picture);
        self.apply_frame_seek(command, &picture);
        true
    }

    fn apply_animation_state(&self, command: &Pending, picture: &Picture) {
        let mut pb = picture.playback.write();
        match command.number(b's') {
            1 => {
                pb.paused_frame = picture.current_frame_unlocked(&pb);
                pb.running = false;
            }
            2 | 3 => {
                pb.running = true;
                *picture.started.write() = Instant::now();
            }
            _ => {}
        }
        match command.number(b'v') {
            1 => pb.loops = 0,
            n if n > 1 => pb.loops = n - 1,
            _ => {}
        }
    }

    fn apply_frame_seek(&self, command: &Pending, picture: &Picture) {
        let mut anim = picture.animation.write();
        if let Some(r) = command.number(b'r').checked_sub(1)
            && let Some(frame) = anim.frames.get_mut(r as usize)
            && let Some(z) = command.signed(b'z')
        {
            frame.delay = if z <= 0 { 1 } else { z as u32 };
        }
        if let Some(c) = command.number(b'c').checked_sub(1)
            && (c as usize) < anim.frames.len()
        {
            picture.playback.write().paused_frame = c as usize;
            let offset: u64 = anim.frames[..c as usize]
                .iter()
                .map(|f| u64::from(f.delay))
                .sum();
            *picture.started.write() = Instant::now() - Duration::from_millis(offset);
        }
    }

    // заглавная буква вдобавок освобождает данные картинки
    fn kitty_delete(&mut self, command: &Pending) {
        let target = command.letter(b'd', b'a');
        let id = (command.number(b'i'), command.number(b'p'));
        let by_id = target.eq_ignore_ascii_case(&b'i');
        if by_id {
            self.remove_images(|p| p.id.0 == id.0 && (id.1 == 0 || p.id.1 == id.1));
            self.graphics.remove_virtual(id.0, id.1);
        } else if target.eq_ignore_ascii_case(&b'a') {
            self.remove_images(|_| true);
            self.graphics.remove(None);
        }
        if target.is_ascii_uppercase() {
            self.graphics.remove(by_id.then_some(id.0));
        }
    }

    // q=1 подавляет успех, q=2 — и ошибки; без номера отвечать некому
    fn kitty_reply(&mut self, command: &Pending, ok: bool) {
        let (id, quiet) = (command.number(b'i'), command.number(b'q'));
        if id == 0 || quiet >= 2 || (ok && quiet == 1) {
            return;
        }
        let status = if ok { "OK" } else { "EINVAL:cannot process" };
        let placement = match command.number(b'p') {
            0 => String::new(),
            p => format!(",p={p}"),
        };
        self.answer(format!("\x1b_Gi={id}{placement};{status}\x1b\\"));
    }
}

impl Picture {
    fn current_frame_unlocked(&self, pb: &super::Playback) -> usize {
        if !pb.running {
            return pb.paused_frame;
        }
        self.animation
            .read()
            .frame_at_limit(self.started.read().elapsed(), pb.loops)
            .0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_and_device_files_are_never_opened() {
        for path in [
            "/proc/self/environ",
            "/sys/kernel/uevent_seqnum",
            "/dev/null",
        ] {
            assert!(open_checked(Path::new(path)).is_none(), "{path}");
        }
        let link = std::env::temp_dir().join(format!("dimaster-link-{}", std::process::id()));
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("/proc/self/environ", &link).unwrap();
        assert!(open_checked(&link).is_none(), "ссылка в /proc");
        std::fs::remove_file(link).unwrap();
        assert!(open_checked(Path::new("/etc/hostname")).is_some());
    }
}
