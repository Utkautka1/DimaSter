//! Очистка строк, пришедших от программ.
//!
//! Такие строки потом попадают в заголовок окна, уведомления, плагины и даже
//! обратно в shell, поэтому в них не должно быть управляющих символов, а
//! длина ограничена.

// предел на заголовок, путь, уведомление и текст команды, в байтах
const LIMIT: usize = 4096;

pub(crate) fn clean(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars().filter(|c| !c.is_control()) {
        if out.len() + ch.len_utf8() > LIMIT {
            break;
        }
        out.push(ch);
    }
    out
}
