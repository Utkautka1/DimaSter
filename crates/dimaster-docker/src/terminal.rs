//! Цикл на настоящем терминале: альтернативный экран, клавиши и передача
//! терминала программе внутри контейнера.

use std::io::{self, Write};
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute, queue};

use crate::app::App;
use crate::host::allow_prompts;
use crate::render;

fn draw(app: &mut App, out: &mut impl Write) -> io::Result<()> {
    let (width, height) = terminal::size()?;
    let lines = render::frame(app, width.into(), height.into());
    // синхронный вывод: кадр появляется целиком, без мерцания
    out.write_all(b"\x1b[?2026h")?;
    for (row, line) in (0..height).zip(&lines) {
        queue!(out, cursor::MoveTo(0, row))?;
        write!(out, "{line}\x1b[K")?;
    }
    out.write_all(b"\x1b[?2026l")?;
    out.flush()
}

fn enter(out: &mut impl Write) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    execute!(out, EnterAlternateScreen, cursor::Hide)
}

fn leave(out: &mut impl Write) -> io::Result<()> {
    let restored = execute!(out, cursor::Show, LeaveAlternateScreen);
    terminal::disable_raw_mode()?;
    restored
}

// shell внутри контейнера получает обычный терминал, потом экран возвращается
fn attach(app: &mut App, args: &[String], out: &mut impl Write) -> io::Result<()> {
    leave(out)?;
    let mut command = app.host().command(args);
    allow_prompts(&mut command);
    match command.status() {
        // код выхода самого shell — не ошибка экрана
        Ok(status) if status.code() == Some(125) => app.message = status.to_string(),
        Ok(_) => {}
        Err(e) => app.message = e.to_string(),
    }
    app.refresh();
    enter(out)
}

fn work(app: &mut App, out: &mut impl Write) -> io::Result<()> {
    let mut stale = true;
    while !app.done {
        if stale {
            draw(app, out)?;
        }
        // просыпаемся и без нажатий: списки и логи приходят из потоков
        stale = event::poll(Duration::from_millis(100))?;
        if stale {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => app.press(key),
                _ => {}
            }
        }
        if let Some(args) = app.attach.take() {
            attach(app, &args, out)?;
        }
        stale |= app.poll();
    }
    Ok(())
}

/// Показывает экран до выхода; терминал восстанавливается и после ошибки.
pub fn run(mut app: App) -> io::Result<()> {
    let mut out = io::stdout();
    enter(&mut out)?;
    let result = work(&mut app, &mut out);
    leave(&mut out).and(result)
}
