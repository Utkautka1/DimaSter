//! OSC: заголовок окна, рабочий каталог и shell-интеграция.

use dimaster_image::decode_base64;

use crate::PromptMark;
use crate::terminal::State;

// больше мегабайта в буфер обмена из программы не принимаем
const CLIPBOARD_LIMIT: usize = 1 << 20;

impl State {
    pub(crate) fn handle_osc(&mut self, data: &[u8]) {
        // картинки идут мегабайтами base64 — не перекодируем их в строку
        if let Some(payload) = data.strip_prefix(b"1337;") {
            return self.iterm(payload);
        }
        let text = String::from_utf8_lossy(data);
        let Some((code, value)) = text.split_once(';') else {
            // сбросы цветов приходят без аргументов
            return match text.as_ref() {
                code @ ("104" | "110" | "111" | "112") => self.reset_colors(code, ""),
                _ => log::debug!("OSC без аргументов: {text}"),
            };
        };
        match code {
            "0" | "2" => self.title = value.to_owned(),
            "7" => self.cwd = parse_file_url(value),
            "4" => self.palette_osc(value),
            "10" | "11" | "12" => self.dynamic_color(code, value),
            "104" | "110" | "111" | "112" => self.reset_colors(code, value),
            "52" => self.clipboard_osc(value),
            "9" => self.notify("DimaSter", value),
            "777" => self.notify_titled(value),
            "133" => self.mark_prompt(value),
            _ => log::debug!("неподдерживаемый OSC {code}"),
        }
    }

    // OSC 9;4;… — это индикатор прогресса ConEmu, а не уведомление
    fn notify(&mut self, title: &str, body: &str) {
        let progress = body.starts_with("4;");
        if !progress && self.notifications.len() < 8 {
            self.notifications.push((title.to_owned(), body.to_owned()));
        }
    }

    // OSC 777;notify;заголовок;текст
    fn notify_titled(&mut self, value: &str) {
        if let Some((title, body)) = value
            .strip_prefix("notify;")
            .and_then(|v| v.split_once(';'))
        {
            self.notify(title, body);
        }
    }

    // OSC 52;<куда>;<base64>. Чтение буфера (`?`) не выполняется: иначе любая
    // программа на сервере могла бы забрать пароль из буфера обмена
    fn clipboard_osc(&mut self, value: &str) {
        let Some((target, data)) = value.split_once(';') else {
            return;
        };
        let decoded = (data != "?" && data.len() <= CLIPBOARD_LIMIT)
            .then(|| decode_base64(data.as_bytes()))
            .flatten()
            .and_then(|bytes| String::from_utf8(bytes).ok());
        if let Some(text) = decoded
            && self.clipboard.len() < 8
        {
            self.clipboard.push((target.contains('p'), text));
        }
    }

    fn mark_prompt(&mut self, value: &str) {
        let mark = match value.chars().next() {
            Some('A') => PromptMark::Prompt,
            Some('C') => PromptMark::Output,
            _ => return,
        };
        self.screen.rows[self.cursor.row].mark = Some(mark);
    }
}

// OSC 7 присылает file://host/path с процентным кодированием
fn parse_file_url(url: &str) -> Option<String> {
    let rest = url.strip_prefix("file://")?;
    let path = &rest[rest.find('/')?..];
    Some(percent_decode(path.as_bytes()))
}

fn percent_decode(mut input: &[u8]) -> String {
    let mut out = Vec::with_capacity(input.len());
    while let Some((&byte, rest)) = input.split_first() {
        let decoded = (byte == b'%')
            .then(|| rest.get(..2))
            .flatten()
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        out.push(decoded.unwrap_or(byte));
        input = if decoded.is_some() { &rest[2..] } else { rest };
    }
    String::from_utf8_lossy(&out).into_owned()
}
