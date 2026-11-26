//! Состояние открытого окна терминала.

use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dimaster_font::FontSystem;
use dimaster_grid::{Size, Terminal};
use dimaster_pty::WindowSize;
use dimaster_render::{PADDING, Renderer};
use winit::event::KeyEvent;
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::Window;

use crate::app::is_command;
use crate::keys;
use crate::session::Session;

// 12 pt при 96 DPI; настройка появится в Lua-конфиге
const FONT_SIZE: f64 = 16.0;
// столько порций разбираем за раз, чтобы поток вывода не блокировал ввод
const BATCH: usize = 32;

// порядок совпадает с алфавитом в latin_letter
const LETTER_KEYS: [KeyCode; 26] = {
    use KeyCode::*;
    [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    ]
};

// композитор присылает окончательные размер и масштаб не сразу после создания окна
const SETTLE: Duration = Duration::from_millis(60);

pub(crate) fn font_pixels(scale: f64) -> f32 {
    (FONT_SIZE * scale) as f32
}

/// Окно, эмулятор и shell.
pub(crate) struct View {
    window: Arc<Window>,
    renderer: Renderer,
    fonts: FontSystem,
    term: Terminal,
    // None, пока размер окна не устоялся
    session: Option<Session>,
    args: Vec<String>,
    start_at: Instant,
    pub(crate) modifiers: ModifiersState,
    // на сколько строк отмотана история
    scroll: usize,
    // накопленная дробная часть прокрутки тачпада
    wheel_rest: f64,
    clipboard: Option<arboard::Clipboard>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

fn grid_size(window: &Window, fonts: &FontSystem) -> Size {
    let size = window.inner_size();
    let metrics = fonts.metrics();
    let cells = |pixels: u32, cell: u32| (pixels.saturating_sub(PADDING * 2) / cell) as usize;
    Size {
        rows: cells(size.height, metrics.cell_height).max(1),
        cols: cells(size.width, metrics.cell_width).max(2),
    }
}

fn window_size(size: Size) -> WindowSize {
    let clamp = |v: usize| u16::try_from(v).unwrap_or(u16::MAX);
    WindowSize {
        rows: clamp(size.rows),
        cols: clamp(size.cols),
    }
}

impl View {
    pub(crate) fn new(
        window: Arc<Window>,
        renderer: Renderer,
        fonts: FontSystem,
        args: &[String],
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let clipboard = arboard::Clipboard::new()
            .inspect_err(|e| log::warn!("Буфер обмена недоступен: {e}"))
            .ok();
        let term = Terminal::new(grid_size(&window, &fonts));
        window.request_redraw();
        Self {
            window,
            renderer,
            fonts,
            term,
            session: None,
            args: args.to_vec(),
            start_at: Instant::now() + SETTLE,
            modifiers: ModifiersState::empty(),
            scroll: 0,
            wheel_rest: 0.0,
            clipboard,
            wake: Arc::new(wake),
        }
    }

    /// Запускает shell, когда размер устоялся: иначе программы из автозапуска
    /// (fastfetch) печатают под временную ширину.
    ///
    /// Возвращает момент, в который нужно вызвать метод снова.
    pub(crate) fn start_when_settled(&mut self) -> io::Result<Option<Instant>> {
        if self.session.is_some() {
            return Ok(None);
        }
        if Instant::now() < self.start_at {
            return Ok(Some(self.start_at));
        }
        let size = grid_size(&self.window, &self.fonts);
        let wake = self.wake.clone();
        self.session = Some(Session::spawn(&self.args, window_size(size), move || {
            wake()
        })?);
        Ok(None)
    }

    fn send(&self, bytes: Vec<u8>) {
        if let Some(session) = &self.session {
            session.send(bytes);
        }
    }

    /// Разбирает накопившийся вывод. `false` — shell завершился.
    pub(crate) fn read_output(&mut self) -> bool {
        let title = self.term.title().to_owned();
        for _ in 0..BATCH {
            let Some(Ok(bytes)) = self.session.as_ref().map(|s| s.output.try_recv()) else {
                break;
            };
            if bytes.is_empty() {
                return false;
            }
            self.term.advance(&bytes);
            let reply = self.term.take_reply();
            self.send(reply);
        }
        if self.term.title() != title {
            self.window.set_title(self.term.title());
        }
        self.window.request_redraw();
        true
    }

    pub(crate) fn draw(&mut self) {
        if !self.renderer.draw(&self.term, &mut self.fonts, self.scroll) {
            (self.wake)();
        }
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
        let size = grid_size(&self.window, &self.fonts);
        self.term.resize(size);
        match &self.session {
            Some(session) => session.resize(window_size(size)),
            None => self.start_at = Instant::now() + SETTLE,
        }
        self.window.request_redraw();
    }

    pub(crate) fn rescale(&mut self, scale: f64) {
        self.fonts.set_size(font_pixels(scale));
        self.renderer.reset_glyphs();
        let size = self.window.inner_size();
        self.resize(size.width, size.height);
    }

    pub(crate) fn key(&mut self, event: &KeyEvent) {
        if self.shortcut(event) {
            return;
        }
        // без модификаторов: Ctrl+C должен работать и на русской раскладке
        let key = self.layout_independent(event);
        let app_cursor = self.term.modes().app_cursor;
        let text = event.text.as_deref();
        if let Some(bytes) = keys::encode(&key, text, self.modifiers, app_cursor) {
            self.type_bytes(bytes);
        }
    }

    fn layout_independent(&self, event: &KeyEvent) -> Key {
        let key = event.key_without_modifiers();
        let latin = matches!(&key, Key::Character(c) if c.is_ascii());
        if !self.modifiers.control_key() || latin {
            return key;
        }
        match event.physical_key {
            PhysicalKey::Code(code) => latin_letter(code).map_or(key, |c| Key::Character(c.into())),
            PhysicalKey::Unidentified(_) => key,
        }
    }

    // сочетания самого терминала: вставка и прокрутка истории
    fn shortcut(&mut self, event: &KeyEvent) -> bool {
        if is_command(self.modifiers) && event.physical_key == PhysicalKey::Code(KeyCode::KeyV) {
            self.paste();
            return true;
        }
        let page = self.term.screen().rows().len() as f64;
        match event.logical_key {
            Key::Named(NamedKey::PageUp) if self.modifiers.shift_key() => self.wheel(page),
            Key::Named(NamedKey::PageDown) if self.modifiers.shift_key() => self.wheel(-page),
            _ => return false,
        }
        true
    }

    fn paste(&mut self) {
        let Some(clipboard) = &mut self.clipboard else {
            return;
        };
        match clipboard.get_text() {
            Ok(text) => {
                let bytes = keys::paste(&text, self.term.modes().bracketed_paste);
                self.type_bytes(bytes);
            }
            Err(e) => log::debug!("в буфере обмена нет текста: {e}"),
        }
    }

    /// Ввод пользователя всегда возвращает вид к последней строке.
    pub(crate) fn type_bytes(&mut self, bytes: Vec<u8>) {
        self.scroll = 0;
        self.send(bytes);
        self.window.request_redraw();
    }

    pub(crate) fn wheel(&mut self, lines: f64) {
        self.wheel_rest += lines;
        let whole = self.wheel_rest.trunc();
        self.wheel_rest -= whole;
        let max = self.term.screen().history().count();
        self.scroll = self.scroll.saturating_add_signed(whole as isize).min(max);
        self.window.request_redraw();
    }

    pub(crate) fn focus(&self, focused: bool) {
        if self.term.modes().focus_events {
            let report: &[u8] = if focused { b"\x1b[I" } else { b"\x1b[O" };
            self.send(report.to_vec());
        }
    }
}

/// Латинская буква физической клавиши при любой раскладке.
fn latin_letter(code: KeyCode) -> Option<&'static str> {
    let index = LETTER_KEYS.iter().position(|&c| c == code)?;
    "abcdefghijklmnopqrstuvwxyz".get(index..=index)
}
