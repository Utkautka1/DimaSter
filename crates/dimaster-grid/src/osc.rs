//! OSC: заголовок окна, рабочий каталог и shell-интеграция.

use dimaster_i18n::t;
use dimaster_image::decode_base64;

use crate::PromptMark;
use crate::terminal::State;
use crate::untrusted::clean;

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
            // сбросы приходят без аргументов
            return match text.parse() {
                Ok(code @ (104 | 105 | 110..=119)) => self.reset_colors(code, ""),
                _ => log::debug!("{}", t!("OSC без аргументов: {text}", text = text)),
            };
        };
        match code {
            "0" | "2" if self.policy.title => self.title = clean(value),
            // значок окна не показывается; запрещённый заголовок молча пропускается
            "0" | "1" | "2" => {}
            "8" => self.hyperlink(value),
            "7" => self.cwd = parse_file_url(value).map(|path| clean(&path)),
            "22" => self.pointer_shape(value),
            "52" => self.clipboard_osc(value),
            "9" => self.conemu(value),
            "777" => self.notify_titled(value),
            "133" => self.mark_prompt(value),
            "633" => self.command_line(value),
            _ => self.color_osc(code, value),
        }
    }

    fn color_osc(&mut self, code: &str, value: &str) {
        match code.parse() {
            Ok(4) if self.policy.colors => self.palette_osc(value),
            Ok(5) if self.policy.colors => self.special_colors(value),
            Ok(code @ 10..=19) if self.policy.colors => self.dynamic_colors(code, value),
            Ok(code @ (104 | 105 | 110..=119)) => self.reset_colors(code, value),
            Ok(4 | 5 | 10..=19) => {}
            _ => log::debug!("{}", t!("неподдерживаемый OSC {code}", code = code)),
        }
    }

    pub(crate) fn notify(&mut self, title: &str, body: &str) {
        if self.policy.notifications && self.notifications.len() < 8 {
            self.notifications.push((clean(title), clean(body)));
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

    // OSC 633;E;<текст> — какую команду сейчас запустят
    fn command_line(&mut self, value: &str) {
        if let Some(text) = value.strip_prefix("E;") {
            self.command_typed(text);
        }
    }

    fn mark_prompt(&mut self, value: &str) {
        match value.chars().next() {
            Some('A') => self.commands.input_start = None,
            Some('B') => {
                if !self.alternate {
                    self.commands.input_start = Some((
                        self.screen.evicted + (self.screen.history.len() + self.cursor.row) as u64,
                        self.cursor.col,
                    ));
                }
                return;
            }
            Some('C') => self.command_started(),
            Some('D') => {
                let code = value.split(';').nth(1).and_then(|c| c.parse().ok());
                return self.command_finished(code.unwrap_or(0));
            }
            _ => {}
        }
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
