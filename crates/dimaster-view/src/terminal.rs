//! Цикл на настоящем терминале: альтернативный экран, клавиши и копирование
//! через OSC 52.

use std::io::{self, Write};

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute, queue};
use dimaster_image::encode_base64;

use crate::app::App;
use crate::render;

fn draw(app: &mut App, out: &mut impl Write) -> io::Result<()> {
    let (width, height) = terminal::size()?;
    let lines = render::frame(app, width.into(), height.into());
    // синхронный вывод: кадр появляется целиком, без мерцания
    out.write_all(b"\x1b[?2026h")?;
    for (row, line) in (0..).zip(&lines) {
        queue!(out, cursor::MoveTo(0, row))?;
        write!(out, "{line}\x1b[K")?;
    }
    if let Some(text) = app.copied.take() {
        write!(out, "\x1b]52;c;{}\x07", encode_base64(text.as_bytes()))?;
    }
    out.write_all(b"\x1b[?2026l")?;
    out.flush()
}

fn work(app: &mut App, out: &mut impl Write) -> io::Result<()> {
    while !app.done {
        draw(app, out)?;
        if let Event::Key(key) = event::read()?
            && key.kind != KeyEventKind::Release
        {
            app.press(key);
        }
    }
    Ok(())
}

/// Показывает таблицы до выхода; терминал восстанавливается и после ошибки.
pub fn run(mut app: App) -> io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut out = io::stdout().lock();
    let entered = execute!(out, EnterAlternateScreen, cursor::Hide);
    let result = entered.and_then(|()| work(&mut app, &mut out));
    let restored = execute!(out, cursor::Show, LeaveAlternateScreen);
    terminal::disable_raw_mode()?;
    result.and(restored)
}
