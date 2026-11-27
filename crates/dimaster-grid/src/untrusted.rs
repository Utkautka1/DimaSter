//! Очистка строк, пришедших от программ.
//!
//! Такие строки потом попадают в заголовок окна, уведомления, плагины и даже
//! обратно в shell, поэтому в них не должно быть управляющих символов, а
//! длина ограничена.

pub(crate) fn clean(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars().filter(|c| !c.is_control()) {
        if out.len() + ch.len_utf8() > dimaster_limits::SHELL_TEXT_BYTES.size() {
            break;
        }
        out.push(ch);
    }
    out
}
