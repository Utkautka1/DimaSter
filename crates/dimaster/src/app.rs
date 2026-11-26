//! Окно: связывает события winit, сессию и рендер.

use std::error::Error;
use std::sync::Arc;

use dimaster_config::{Config, Decorations};
use dimaster_font::FontSystem;
use dimaster_render::{Renderer, wgpu};
use notify::RecommendedWatcher;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{Ime, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::platform::wayland::WindowAttributesExtWayland;
use winit::window::{Window, WindowId};

use crate::remote::{self, Request, Server};
use crate::restore::take_saved;
use crate::settings;
use crate::store::Saved;
use crate::view::View;

/// События из фоновых потоков.
#[derive(Debug)]
pub(crate) enum Wake {
    /// Есть вывод shell или нужен повторный кадр.
    Output,
    ConfigChanged,
    /// Команда из `dimaster ctl`.
    Remote(Request),
}

/// Открывает окно и работает, пока оно или shell не закроются.
pub(crate) fn run(
    args: Vec<String>,
    config: Config,
    restorable: bool,
) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::<Wake>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let changed = proxy.clone();
    let watcher = settings::watch(move || drop(changed.send_event(Wake::ConfigChanged)));
    let control = proxy.clone();
    let server = config
        .remote_control
        .then(|| remote::listen(move |r| drop(control.send_event(Wake::Remote(r)))))
        .flatten();
    let others = remote::live_sockets().len() > usize::from(server.is_some());
    // команда после -e восстановления не требует
    let restore = (restorable && config.restore_session)
        .then(|| take_saved(others))
        .flatten();
    let mut app = App {
        args,
        config,
        proxy,
        view: None,
        error: None,
        _watcher: watcher,
        server,
        restore,
    };
    event_loop.run_app(&mut app)?;
    app.error.map_or(Ok(()), Err)
}

// OpenGL подгружается, только если Vulkan не нашёлся: оба сразу — это лишние
// ~20 МБ памяти на каждое окно
fn renderer(
    event_loop: &ActiveEventLoop,
    window: &Arc<Window>,
) -> Result<Renderer, Box<dyn Error>> {
    // явный выбор пользователя через WGPU_BACKEND не переопределяется
    if std::env::var_os("WGPU_BACKEND").is_none() {
        match renderer_with(event_loop, window, wgpu::Backends::VULKAN) {
            Ok(renderer) => return Ok(renderer),
            Err(e) => log::warn!("Vulkan недоступен, пробуем OpenGL: {e}"),
        }
    }
    renderer_with(event_loop, window, wgpu::Backends::all())
}

fn renderer_with(
    event_loop: &ActiveEventLoop,
    window: &Arc<Window>,
    backends: wgpu::Backends,
) -> Result<Renderer, Box<dyn Error>> {
    let display = Box::new(event_loop.owned_display_handle());
    let mut descriptor = wgpu::InstanceDescriptor::new_with_display_handle_from_env(display);
    descriptor.backends &= backends;
    let instance = wgpu::Instance::new(descriptor);
    let surface = instance.create_surface(window.clone())?;
    let size = window.inner_size();
    Ok(Renderer::new(instance, surface, (size.width, size.height))?)
}

struct App {
    args: Vec<String>,
    config: Config,
    proxy: EventLoopProxy<Wake>,
    view: Option<View>,
    // ошибка запуска: цикл событий сам вернуть её не может
    error: Option<Box<dyn Error>>,
    // слежение за конфигом живо, пока жив объект
    _watcher: Option<RecommendedWatcher>,
    // сокет живёт вместе с приложением
    server: Option<Server>,
    restore: Option<Saved>,
}

impl App {
    fn open(&self, event_loop: &ActiveEventLoop) -> Result<View, Box<dyn Error>> {
        let attributes = Window::default_attributes()
            .with_title("DimaSter")
            .with_decorations(self.config.window.decorations == Decorations::System)
            .with_transparent(true)
            .with_blur(self.config.window.blur)
            // app_id нужен правилам окон композитора
            .with_name("dimaster", "dimaster");
        let window = Arc::new(event_loop.create_window(attributes)?);
        window.set_ime_allowed(true);
        let renderer = renderer(event_loop, &window)?;
        let config = self.config.clone();
        let fonts = settings::fonts(&config, window.scale_factor())?;
        let proxy = self.proxy.clone();
        let wake = move || drop(proxy.send_event(Wake::Output));
        request_size(&window, &config, &fonts);
        Ok(View::new(window, renderer, fonts, config, &self.args, wake))
    }
}

impl App {
    // последнее окно сохраняет сессию, остальные считаются закрытыми намеренно
    fn close(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(view) = &mut self.view {
            let own = usize::from(self.server.is_some());
            view.close_session(remote::live_sockets().len() <= own);
        }
        event_loop.exit();
    }
}

impl ApplicationHandler<Wake> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.view.is_some() {
            return;
        }
        match self.open(event_loop) {
            Ok(mut view) => {
                view.socket = self.server.as_ref().map(|s| s.path.clone());
                if let Some(saved) = self.restore.take() {
                    view.restore(saved);
                }
                view.load_plugins();
                self.view = Some(view);
            }
            Err(e) => {
                log::error!("Не удалось открыть окно: {e}");
                self.error = Some(e);
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(view) = &mut self.view else { return };
        view.autosave();
        match view.start_when_settled() {
            Ok(Some(at)) => event_loop.set_control_flow(ControlFlow::WaitUntil(at)),
            Ok(None) => {
                let flow = view.refresh_deadline();
                event_loop.set_control_flow(flow.map_or(ControlFlow::Wait, ControlFlow::WaitUntil));
            }
            Err(e) => {
                log::error!("Не удалось запустить shell: {e}");
                self.error = Some(e.into());
                event_loop.exit();
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: Wake) {
        let Some(view) = &mut self.view else { return };
        match event {
            Wake::ConfigChanged => {
                view.reload();
                view.load_plugins();
            }
            Wake::Remote(request) => view.remote(request),
            // shell завершился сам — восстанавливать нечего
            Wake::Output if !view.read_output() => {
                view.close_session(false);
                event_loop.exit();
            }
            Wake::Output => {}
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(view) = &mut self.view else { return };
        match event {
            WindowEvent::CloseRequested => self.close(event_loop),
            WindowEvent::RedrawRequested => view.draw(),
            WindowEvent::Resized(size) => view.resize(size.width, size.height),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => view.rescale(scale_factor),
            WindowEvent::ModifiersChanged(mods) => view.modifiers = mods.state(),
            WindowEvent::KeyboardInput { event, .. } if event.state.is_pressed() => {
                view.key(&event);
            }
            WindowEvent::Ime(Ime::Preedit(text, _)) => view.set_preedit(text),
            WindowEvent::Ime(Ime::Commit(text)) => view.type_bytes(text.into_bytes()),
            WindowEvent::CursorMoved { position, .. } => {
                view.pointer_moved(position.x, position.y);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                view.pointer_button(state, button);
                if view.closing {
                    self.close(event_loop);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => view.wheel(lines(delta)),
            WindowEvent::Focused(focused) => view.focus(focused),
            _ => {}
        }
    }
}

// начальный размер из настроек; тайловый композитор вправе его не принять
fn request_size(window: &Window, config: &Config, fonts: &FontSystem) {
    let (Some(columns), Some(rows)) = (config.window.columns, config.window.rows) else {
        return;
    };
    let metrics = fonts.metrics();
    let width = columns * metrics.cell_width + config.padding.0 * 2;
    let height = rows * metrics.cell_height
        + config.padding.1 * 2
        + settings::title_height(config, metrics.cell_height);
    let _ = window.request_inner_size(PhysicalSize::new(width, height));
}

// положительное значение — прокрутка к истории
fn lines(delta: MouseScrollDelta) -> f64 {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => f64::from(y),
        // тачпад отдаёт пиксели; 20 px ≈ одна строка
        MouseScrollDelta::PixelDelta(position) => position.y / 60.0,
    }
}
