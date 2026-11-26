//! Headless-режим первого этапа: прогоняет команду через эмулятор и печатает экран.
//!
//! Позволяет проверить ядро на реальных программах до появления окна.

use std::io::{self, Read, Write};
use std::process::{Command, ExitCode};

use dimaster_grid::{Size, Terminal};
use dimaster_pty::{Pty, WindowSize};

const SIZE: WindowSize = WindowSize {
    rows: 40,
    cols: 120,
};

fn main() -> ExitCode {
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("использование: dimaster <команда> [аргументы]");
        return ExitCode::from(2);
    }
    match run(&args) {
        Ok(screen) => {
            println!("{screen}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            log::error!("не удалось выполнить {}: {e}", args[0]);
            eprintln!("ошибка: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Возвращает историю и видимый экран после завершения команды.
fn run(args: &[String]) -> io::Result<String> {
    let mut command = Command::new(&args[0]);
    // до появления своего terminfo притворяемся xterm
    command.args(&args[1..]).env("TERM", "xterm-256color");
    let mut pty = Pty::spawn(command, SIZE)?;
    let mut term = Terminal::new(Size {
        rows: SIZE.rows.into(),
        cols: SIZE.cols.into(),
    });
    pump(&mut pty, &mut term)?;
    pty.wait()?;
    let mut lines: Vec<String> = term.screen().history().map(|r| r.text()).collect();
    lines.push(term.text());
    Ok(lines.join("\n"))
}

fn pump(pty: &mut Pty, term: &mut Terminal) -> io::Result<()> {
    let mut buf = [0; 64 * 1024];
    loop {
        let read = pty.read(&mut buf)?;
        if read == 0 {
            return Ok(());
        }
        term.advance(&buf[..read]);
        pty.write_all(&term.take_reply())?;
    }
}
