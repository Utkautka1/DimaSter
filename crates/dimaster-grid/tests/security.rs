//! Всё, что приходит из вывода программ, считается враждебным: это может быть
//! чужой сервер или просто файл, показанный через `cat`.

use dimaster_grid::{Size, SmartOptions, Terminal};
use dimaster_image::encode_base64;

fn run(input: &[u8]) -> Terminal {
    let mut term = Terminal::new(Size { rows: 10, cols: 40 });
    term.set_smart_options(SmartOptions::default());
    term.advance(input);
    term
}

// ответ уходит в shell как набранный текст — переводов строк в нём быть не должно
#[test]
fn replies_never_carry_request_text_into_the_shell() {
    let mut term = run(b"\x1bP+q\rtouch /tmp/pwned\r\x1b\\");
    let reply = term.take_reply();
    assert!(
        !reply.contains(&b'\r') && !reply.contains(&b'\n'),
        "{reply:?}"
    );
    assert!(!String::from_utf8_lossy(&reply).contains("pwned"));
}

// текст команды вставляется обратно из истории и через Sudo Fix
#[test]
fn forged_command_text_cannot_press_enter() {
    let term = run(b"\x1b]633;E;rm -rf x\rwhoami\n\x07\x1b]133;C\x07\x1b]133;D;0\x07");
    let command = &term.commands().next().unwrap().command;
    assert!(
        command.starts_with("rm -rf x"),
        "команда не дошла: {command:?}"
    );
    assert!(!command.chars().any(char::is_control), "{command:?}");
}

#[test]
fn strings_from_programs_are_bounded() {
    let long = "x".repeat(1 << 20);
    let input = format!("\x1b]0;{long}\x07\x1b]7;file://h/{long}\x07\x1b]9;{long}\x07");
    let mut term = run(input.as_bytes());
    assert!(term.title().len() <= 4096);
    assert!(term.cwd().unwrap().len() <= 4096);
    let notifications = term.take_notifications();
    assert!(notifications.iter().all(|n| n.1.len() <= 4096));
}

#[test]
fn graphics_cannot_delete_arbitrary_files() {
    // имя содержит метку протокола, но файл лежит не во временном каталоге
    let directory = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
    let victim = directory.join("important-tty-graphics-protocol.txt");
    std::fs::write(&victim, "data").unwrap();
    let path = encode_base64(victim.to_str().unwrap().as_bytes());
    run(format!("\x1b_Ga=T,t=t,f=100;{path}\x1b\\").as_bytes());
    assert!(victim.exists(), "файл удалён по команде из вывода");
    std::fs::remove_file(victim).unwrap();
}

#[test]
fn pictures_have_a_total_memory_budget() {
    // 200 картинок по 1000x1000 = 800 МБ, если ничего не ограничивать
    let mut term = run(b"");
    for _ in 0..200 {
        term.advance(b"\x1bPq\"1;1;1000;1000#1~\x1b\\\r\n");
    }
    let bytes: usize = term
        .screen()
        .history()
        .flat_map(|r| r.images)
        .chain(term.screen().rows().iter().flat_map(|r| r.images.clone()))
        .map(|p| p.picture.bytes())
        .sum();
    assert!(bytes <= 320 << 20, "{} МБ", bytes >> 20);
}
