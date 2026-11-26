//! OSC: заголовок окна, рабочий каталог и shell-интеграция.

use crate::PromptMark;
use crate::terminal::State;

impl State {
    pub(crate) fn handle_osc(&mut self, data: &[u8]) {
        // картинки идут мегабайтами base64 — не перекодируем их в строку
        if let Some(payload) = data.strip_prefix(b"1337;") {
            return self.iterm(payload);
        }
        let text = String::from_utf8_lossy(data);
        let Some((code, value)) = text.split_once(';') else {
            return log::debug!("OSC без аргументов: {text}");
        };
        match code {
            "0" | "2" => self.title = value.to_owned(),
            "7" => self.cwd = parse_file_url(value),
            "10" | "11" if value == "?" => self.report_color(code),
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
