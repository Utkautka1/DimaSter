//! Цикл на настоящем терминале: альтернативный экран, клавиши и копирование
//! через OSC 52.

use std::io::{self, Write};
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute, queue};
use dimaster_image::encode_base64;

use crate::app::App;
use crate::remote::{self, Server};
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

// выполняет присланное; `true` — было что выполнять
fn serve(app: &mut App, server: Option<&Server>) -> bool {
    let mut served = false;
    while let Some(request) = server.and_then(|s| s.requests.try_recv().ok()) {
        let answer = app.remote(&request.line);
        request.answer(&answer);
        served = true;
    }
    served
}

fn work(app: &mut App, out: &mut impl Write, server: Option<&Server>) -> io::Result<()> {
    let mut stale = true;
    while !app.done {
        if stale {
            draw(app, out)?;
        }
        // с сокетом приходится просыпаться и без нажатий
        stale = server.is_none() || event::poll(Duration::from_millis(100))?;
        if stale {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => app.press(key),
                _ => {}
            }
        }
        stale |= serve(app, server);
    }
    Ok(())
}

/// Показывает таблицы до выхода; терминал восстанавливается и после ошибки.
/// `remote` открывает сокет для `--send`.
pub fn run(mut app: App, remote: bool) -> io::Result<()> {
    let server = remote.then(remote::listen).transpose()?;
    terminal::enable_raw_mode()?;
    let mut out = io::stdout().lock();
    let entered = execute!(out, EnterAlternateScreen, cursor::Hide);
    let result = entered.and_then(|()| work(&mut app, &mut out, server.as_ref()));
    let restored = execute!(out, cursor::Show, LeaveAlternateScreen);
    terminal::disable_raw_mode()?;
    result.and(restored)
}
