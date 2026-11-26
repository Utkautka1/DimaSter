//! Состояние открытого окна терминала.

use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dimaster_config::Config;
use dimaster_font::FontSystem;
use dimaster_grid::{Size, Terminal};
use dimaster_pty::WindowSize;
use dimaster_render::{Renderer, Viewport};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::keyboard::ModifiersState;
use winit::window::Window;

use crate::clipboard::Clipboard;
use crate::pointer::Pointer;
use crate::select::Selecting;
use crate::session::Session;

// столько порций разбираем за раз, чтобы поток вывода не блокировал ввод
const BATCH: usize = 32;
// дольше этого синхронизированный вывод не задерживает кадр: программа могла упасть
const SYNC_LIMIT: Duration = Duration::from_millis(150);

// композитор присылает окончательные размер и масштаб не сразу после создания окна
const SETTLE: Duration = Duration::from_millis(60);

/// Окно, эмулятор и shell.
pub(crate) struct View {
    pub(crate) window: Arc<Window>,
    pub(crate) renderer: Renderer,
    pub(crate) fonts: FontSystem,
    pub(crate) config: Config,
    // прибавка к кеглю в пунктах от Ctrl+плюс/минус
    pub(crate) zoom: f32,
    pub(crate) config_error: bool,
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
    // текст, который пользователь набирает в IME
    preedit: String,
    wake: Arc<dyn Fn() + Send + Sync>,
}

fn grid_size(window: &Window, fonts: &FontSystem, padding: (u32, u32)) -> Size {
    let size = window.inner_size();
    let metrics = fonts.metrics();
    let cells = |pixels: u32, pad: u32, cell: u32| (pixels.saturating_sub(pad * 2) / cell) as usize;
    Size {
        rows: cells(size.height, padding.1, metrics.cell_height).max(1),
        cols: cells(size.width, padding.0, metrics.cell_width).max(2),
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
        config: Config,
        args: &[String],
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        let term = Terminal::new(grid_size(&window, &fonts, config.padding));
        window.request_redraw();
        let mut view = Self {
            window,
            renderer,
            fonts,
            config,
            zoom: 0.0,
            config_error: false,
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
            preedit: String::new(),
            wake: Arc::new(wake),
        };
        view.apply_to_terminal();
        view
    }

    fn grid_size(&self) -> Size {
        grid_size(&self.window, &self.fonts, self.config.padding)
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
        let size = self.grid_size();
        let wake = self.wake.clone();
        let (term, size) = (&self.config.term, window_size(size));
        self.session = Some(Session::spawn(&self.args, term, size, move || wake())?);
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
            self.update_title();
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
            preedit: &self.preedit,
        };
        if !self.renderer.draw(&self.term, &mut self.fonts, view) {
            (self.wake)();
        }
    }

    pub(crate) fn set_preedit(&mut self, text: String) {
        self.preedit = text;
        self.place_ime_window();
        self.window.request_redraw();
    }

    // окно кандидатов IME должно появляться у курсора
    fn place_ime_window(&self) {
        let (metrics, cursor) = (self.fonts.metrics(), self.term.cursor());
        let at = |cells: usize, size: u32, pad: u32| pad + cells as u32 * size;
        let x = at(cursor.col, metrics.cell_width, self.config.padding.0);
        let y = at(cursor.row, metrics.cell_height, self.config.padding.1);
        let size = PhysicalSize::new(metrics.cell_width, metrics.cell_height);
        self.window
            .set_ime_cursor_area(PhysicalPosition::new(x, y), size);
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
        let size = self.grid_size();
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
        let size = self.grid_size();
        let index = |position: f64, pad: u32, cell: u32, count: usize| {
            let offset = (position - f64::from(pad)).max(0.0);
            ((offset / f64::from(cell)) as usize).min(count - 1)
        };
        (
            index(x, self.config.padding.0, metrics.cell_width, size.cols),
            index(y, self.config.padding.1, metrics.cell_height, size.rows),
        )
    }

    pub(crate) fn focus(&self, focused: bool) {
        if self.term.modes().focus_events {
            let report: &[u8] = if focused { b"\x1b[I" } else { b"\x1b[O" };
            self.send(report.to_vec());
        }
    }
}
