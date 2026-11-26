//! Проверяет, что процесс получает настоящий управляющий терминал.

use std::io::Read;
use std::process::Command;

use dimaster_pty::{Pty, WindowSize};

#[test]
fn child_sees_tty_with_requested_size() {
    let mut command = Command::new("sh");
    command.args(["-c", "stty size < /dev/tty"]);
    let mut pty = Pty::spawn(command, WindowSize { rows: 7, cols: 33 }).unwrap();
    let mut output = String::new();
    pty.read_to_string(&mut output).unwrap();
    assert_eq!(output.trim(), "7 33");
    assert!(pty.wait().unwrap().success());
}
