//! Прогон команды без окна: печатает получившийся экран.

use std::io::{self, Read, Write};

use dimaster_grid::{Size, Terminal};
use dimaster_pty::{Pty, WindowSize};

use crate::session::{Launch, command};

const SIZE: WindowSize = WindowSize {
    rows: 40,
    cols: 120,
};

/// Возвращает историю и видимый экран после завершения команды.
pub(crate) fn run(args: &[String], term_name: &str) -> io::Result<String> {
    let mut pty = Pty::spawn(
        command(&Launch {
            args,
            term_name,
            directory: None,
            socket: None,
            integrate: false,
        }),
        SIZE,
    )?;
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
