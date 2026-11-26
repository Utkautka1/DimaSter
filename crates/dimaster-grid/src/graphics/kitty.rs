//! Графический протокол Kitty (APC `G`).

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use dimaster_image::{Animation, decode, decode_base64, inflate_zlib};

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

// программа вне нашей машины может прислать путь, поэтому только обычные файлы
fn read_file(path: &[u8], temporary: bool) -> Option<Vec<u8>> {
    let path = Path::new(std::str::from_utf8(path).ok()?);
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_PAYLOAD as u64 {
        return None;
    }
    let mut data = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(MAX_PAYLOAD as u64)
        .read_to_end(&mut data)
        .ok()?;
    // удаляем только файлы, явно созданные для протокола
    if temporary && path.to_string_lossy().contains("tty-graphics-protocol") {
        let _ = std::fs::remove_file(path);
    }
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
            b'd' => return self.kitty_delete(command),
            _ => false,
        };
        self.kitty_reply(command, ok);
    }

    fn kitty_transmit(&mut self, command: &Pending, action: u8) -> bool {
        let Some(animation) = load(command) else {
            return false;
        };
        // запрос возможностей: декодировали, но не храним
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

    fn kitty_show(&mut self, command: &Pending, picture: std::sync::Arc<super::Picture>) {
        let size = |key| Some(command.number(key) as usize);
        let cells = self.image_cells(&picture, size(b'c'), size(b'r'));
        let id = (command.number(b'i'), command.number(b'p'));
        // C=1 просит не двигать курсор
        if command.number(b'C') == 1 {
            self.place(picture, cells, id);
        } else {
            self.place_and_advance(picture, cells, id);
        }
    }

    // заглавная буква вдобавок освобождает данные картинки
    fn kitty_delete(&mut self, command: &Pending) {
        let target = command.letter(b'd', b'a');
        let id = (command.number(b'i'), command.number(b'p'));
        let by_id = target.eq_ignore_ascii_case(&b'i');
        if by_id {
            self.remove_images(|p| p.id.0 == id.0 && (id.1 == 0 || p.id.1 == id.1));
        } else if target.eq_ignore_ascii_case(&b'a') {
            self.remove_images(|_| true);
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
        let status = if ok {
            "OK"
        } else {
            "EINVAL:не удалось обработать"
        };
        let placement = match command.number(b'p') {
            0 => String::new(),
            p => format!(",p={p}"),
        };
        self.answer(format!("\x1b_Gi={id}{placement};{status}\x1b\\"));
    }
}
