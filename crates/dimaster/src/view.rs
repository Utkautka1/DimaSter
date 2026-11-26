//! Состояние открытого окна терминала.

use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dimaster_font::FontSystem;
use dimaster_grid::{Size, Terminal};
use dimaster_pty::WindowSize;
use dimaster_render::{PADDING, Renderer, Viewport};
use winit::event::KeyEvent;
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::window::Window;

use crate::app::is_command;
use crate::clipboard::{Clipboard, Target};
use crate::keys;
use crate::pointer::Pointer;
use crate::select::Selecting;
use crate::session::Session;

// 12 pt при 96 DPI; настройка появится в Lua-конфиге
const FONT_SIZE: f64 = 16.0;
// столько порций разбираем за раз, чтобы поток вывода не блокировал ввод
const BATCH: usize = 32;
// дольше этого синхронизированный вывод не задерживает кадр: программа могла упасть
const SYNC_LIMIT: Duration = Duration::from_millis(150);

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
    pub(crate) window: Arc<Window>,
    renderer: Renderer,
    fonts: FontSystem,
    pub(crate) term: Terminal,
    // None, пока размер окна не устоялся
    session: Option<Session>,
    args: Vec<String>,
    start_at: Instant,
    pub(crate) modifiers: ModifiersState,
    // на сколько строк отмотана история
    pub(crate) scroll: usize,
    pub(crate) pointer: Pointer,
    // когда программа включила синхронизированный вывод
    sync_since: Option<Instant>,
    pub(crate) clipboard: Clipboard,
    pub(crate) selecting: Selecting,
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
            pointer: Pointer::default(),
            sync_since: None,
            clipboard: Clipboard::new(),
            selecting: Selecting::default(),
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

    pub(crate) fn send(&self, bytes: Vec<u8>) {
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
            let alternate = self.term.is_alternate();
            self.term.advance(&bytes);
            if alternate != self.term.is_alternate() {
                self.selecting.clear();
            }
            let reply = self.term.take_reply();
            self.send(reply);
        }
        if self.term.title() != title {
            self.window.set_title(self.term.title());
        }
        self.refresh();
        true
    }

    /// Запрашивает кадр, если программа не просила подождать (режим 2026).
    ///
    /// Возвращает крайний срок, к которому кадр нужно показать принудительно.
    pub(crate) fn refresh(&mut self) -> Option<Instant> {
        if self.term.modes().synchronized {
            let since = *self.sync_since.get_or_insert_with(Instant::now);
            if since.elapsed() < SYNC_LIMIT {
                return Some(since + SYNC_LIMIT);
            }
        }
        if self.sync_since.take().is_some() || !self.term.modes().synchronized {
            self.window.request_redraw();
        }
        None
    }

    /// Срок принудительного кадра, пока идёт синхронизированный вывод.
    pub(crate) fn refresh_deadline(&mut self) -> Option<Instant> {
        self.sync_since?;
        self.refresh()
    }

    pub(crate) fn draw(&mut self) {
        let selection = self.selecting.selection.map(|s| self.term.span(s));
        let view = Viewport {
            scroll: self.scroll,
            selection,
        };
        if !self.renderer.draw(&self.term, &mut self.fonts, view) {
            (self.wake)();
        }
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
        let size = grid_size(&self.window, &self.fonts);
        self.term.resize(size);
        // после переразбиения номера строк другие
        self.selecting.clear();
        self.scroll = 0;
        let metrics = self.fonts.metrics();
        self.term
            .set_cell_pixels(metrics.cell_width as usize, metrics.cell_height as usize);
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
        let modes = keys::KeyModes {
            app_cursor: self.term.modes().app_cursor,
            disambiguate: self.term.keyboard_flags() & 1 != 0,
        };
        let text = event.text.as_deref();
        if let Some(bytes) = keys::encode(&key, text, self.modifiers, modes) {
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

    // сочетания самого терминала: копирование, вставка и прокрутка истории
    fn shortcut(&mut self, event: &KeyEvent) -> bool {
        if is_command(self.modifiers) {
            match event.physical_key {
                PhysicalKey::Code(KeyCode::KeyC) => self.copy(Target::Clipboard),
                PhysicalKey::Code(KeyCode::KeyV) => self.paste(Target::Clipboard),
                _ => return false,
            }
            return true;
        }
        let page = self.term.screen().rows().len() as isize;
        match event.logical_key {
            Key::Named(NamedKey::PageUp) if self.modifiers.shift_key() => self.scroll_by(page),
            Key::Named(NamedKey::PageDown) if self.modifiers.shift_key() => self.scroll_by(-page),
            _ => return false,
        }
        true
    }

    /// Ввод пользователя всегда возвращает вид к последней строке.
    pub(crate) fn type_bytes(&mut self, bytes: Vec<u8>) {
        self.scroll = 0;
        self.selecting.clear();
        self.send(bytes);
        self.window.request_redraw();
    }

    pub(crate) fn scroll_by(&mut self, lines: isize) {
        let max = self.term.screen().history().count();
        self.scroll = self.scroll.saturating_add_signed(lines).min(max);
        self.window.request_redraw();
    }

    pub(crate) fn cell_at(&self, x: f64, y: f64) -> (usize, usize) {
        let metrics = self.fonts.metrics();
        let size = grid_size(&self.window, &self.fonts);
        let index = |position: f64, cell: u32, count: usize| {
            let offset = (position - f64::from(PADDING)).max(0.0);
            ((offset / f64::from(cell)) as usize).min(count - 1)
        };
        (
            index(x, metrics.cell_width, size.cols),
            index(y, metrics.cell_height, size.rows),
        )
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
