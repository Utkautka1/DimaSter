//! Поверхность окна, устройство и вывод кадра.

use dimaster_i18n::{tr, trf};
use std::fmt;

use dimaster_font::FontSystem;
use dimaster_grid::Terminal;

use crate::appearance::Appearance;
use crate::atlas::Atlas;
use crate::frame::{Builder, Quad, Viewport};
use crate::images::Images;
use crate::pipeline::Pipeline;
use crate::textures::Textures;

/// Ошибка инициализации графики.
#[derive(Debug)]
pub struct RenderError(String);

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let values = [("error", self.0.as_str())];
        f.write_str(&trf(
            "не удалось инициализировать графику: {error}",
            &values,
        ))
    }
}

impl std::error::Error for RenderError {}

fn fail(error: impl fmt::Display) -> RenderError {
    RenderError(error.to_string())
}

/// Рисует терминал в окне.
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: Pipeline,
    atlas: Atlas,
    look: Appearance,
    // фон текущего кадра с учётом изменений программы
    background: u32,
    quads: Vec<Quad>,
    images: Images,
    overlay: Vec<Quad>,
    // фон кадра: нулевая или одна картинка
    wallpaper: usize,
    textures: Textures,
    instances: wgpu::Buffer,
}

impl Renderer {
    /// `target` — окно; wgpu сам выберет Vulkan или запасной OpenGL.
    pub fn new(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: (u32, u32),
    ) -> Result<Self, RenderError> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            // терминалу хватает встроенной графики, батарея важнее
            power_preference: wgpu::PowerPreference::LowPower,
            ..Default::default()
        }))
        .map_err(fail)?;
        log::info!("Графический адаптер: {:?}", adapter.get_info());
        let descriptor = wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        };
        let (device, queue) =
            pollster::block_on(adapter.request_device(&descriptor)).map_err(fail)?;
        let config = configure(&adapter, &surface, size)?;
        surface.configure(&device, &config);
        Ok(Self::assemble(surface, device, queue, config))
    }

    fn assemble(
        surface: wgpu::Surface<'static>,
        device: wgpu::Device,
        queue: wgpu::Queue,
        config: wgpu::SurfaceConfiguration,
    ) -> Self {
        let pipeline = Pipeline::new(&device, config.format);
        let instances = instance_buffer(&device, 4096);
        let (atlas, quads) = Default::default();
        Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            atlas,
            look: Appearance::default(),
            background: Appearance::default().background,
            quads,
            images: Images::default(),
            overlay: Vec::new(),
            wallpaper: 0,
            textures: Textures::default(),
            instances,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn set_appearance(&mut self, look: Appearance) {
        self.look = look;
    }

    /// Сбрасывает глифы после смены кегля.
    pub fn reset_glyphs(&mut self) {
        self.atlas.clear();
    }

    /// Через сколько перерисовать из-за анимации; `None` — анимаций не видно.
    pub fn next_frame(&self) -> Option<std::time::Duration> {
        self.images.next_frame
    }

    /// Собирает кадр из состояния терминала; после этого он больше не нужен.
    pub fn prepare(&mut self, term: &Terminal, fonts: &mut FontSystem, view: Viewport<'_>) {
        self.build(term, fonts, view);
        if self.atlas.full {
            log::warn!("Атлас глифов переполнен — очищаем");
            self.atlas.clear();
            self.build(term, fonts, view);
        }
        for upload in self.atlas.uploads.drain(..) {
            self.pipeline.upload(&self.queue, &upload);
        }
        let screen = [
            self.config.width as f32,
            self.config.height as f32,
            0.0,
            0.0,
        ];
        self.queue
            .write_buffer(&self.pipeline.screen, 0, bytemuck::cast_slice(&screen));
        self.write_instances();
        let (device, queue) = (&self.device, &self.queue);
        self.textures
            .prepare(device, queue, &self.pipeline, &self.images.quads);
    }

    // порядок в буфере: текст, картинки (обои первыми), панели
    fn write_instances(&mut self) {
        let images = self.images.quads.iter().map(|i| i.quad);
        let text = self.quads.iter().copied();
        let all: Vec<Quad> = text
            .chain(images)
            .chain(self.overlay.iter().copied())
            .collect();
        if self.instances.size() < size_of_val(all.as_slice()) as u64 {
            self.instances = instance_buffer(&self.device, all.len().next_power_of_two());
        }
        self.queue
            .write_buffer(&self.instances, 0, bytemuck::cast_slice(&all));
    }

    fn draw_images(&self, pass: &mut wgpu::RenderPass<'_>, which: std::ops::Range<usize>) {
        let first = self.quads.len();
        for index in which {
            if let Some(bindings) = self.textures.get(&self.images.quads[index]) {
                let at = (first + index) as u32;
                pass.set_bind_group(0, bindings, &[]);
                pass.draw(0..4, at..at + 1);
            }
        }
    }

    // обои, текст, картинки программ, панели
    fn draw_layers(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline.pipeline);
        pass.set_vertex_buffer(0, self.instances.slice(..));
        self.draw_images(pass, 0..self.wallpaper);
        pass.set_bind_group(0, &self.pipeline.bindings, &[]);
        pass.draw(0..4, 0..self.quads.len() as u32);
        self.draw_images(pass, self.wallpaper..self.images.quads.len());
        let start = (self.quads.len() + self.images.quads.len()) as u32;
        pass.set_bind_group(0, &self.pipeline.bindings, &[]);
        pass.draw(0..4, start..start + self.overlay.len() as u32);
    }

    /// Показывает собранный кадр; может ждать монитор, поэтому терминал уже
    /// должен быть отпущен. `false` — кадр пропущен, нужно запросить снова.
    pub fn present(&mut self) -> bool {
        use wgpu::CurrentSurfaceTexture as Current;
        let frame = match self.surface.get_current_texture() {
            Current::Success(frame) | Current::Suboptimal(frame) => frame,
            Current::Outdated | Current::Lost => {
                self.surface.configure(&self.device, &self.config);
                return false;
            }
            // свёрнутое окно придёт в себя по событию от оконной системы
            _ => return true,
        };
        self.submit(&frame.texture.create_view(&Default::default()));
        self.queue.present(frame);
        true
    }

    fn build(&mut self, term: &Terminal, fonts: &mut FontSystem, view: Viewport<'_>) {
        let window = [self.config.width as f32, self.config.height as f32];
        let look = self.look.with_overrides(term.color_overrides());
        self.background = look.background;
        let mut builder = Builder {
            fonts,
            atlas: &mut self.atlas,
            quads: &mut self.quads,
            look: &look,
            pictures: &mut self.images,
        };
        builder.build(term, view);
        let wallpaper = builder.wallpaper(window);
        builder.quads = &mut self.overlay;
        builder.overlay(view.rects, view.labels);
        self.wallpaper = usize::from(wallpaper.is_some());
        if let Some(wallpaper) = wallpaper {
            self.images.quads.insert(0, wallpaper);
        }
    }

    fn submit(&self, view: &wgpu::TextureView) {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let (color, opacity) = (self.background, self.look.opacity);
            let mut pass = begin_pass(&mut encoder, view, color, opacity);
            self.draw_layers(&mut pass);
        }
        self.queue.submit([encoder.finish()]);
    }
}

fn begin_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    background: u32,
    opacity: f32,
) -> wgpu::RenderPass<'a> {
    // поверхность ждёт цвет, уже умноженный на альфу
    let alpha = f64::from(opacity);
    let channel = |shift: u32| f64::from((background >> shift) & 0xff) / 255.0 * alpha;
    let clear = wgpu::Color {
        r: channel(16),
        g: channel(8),
        b: channel(0),
        a: alpha,
    };
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    })
}

fn configure(
    adapter: &wgpu::Adapter,
    surface: &wgpu::Surface<'_>,
    (width, height): (u32, u32),
) -> Result<wgpu::SurfaceConfiguration, RenderError> {
    let mut config = surface
        .get_default_config(adapter, width.max(1), height.max(1))
        .ok_or_else(|| fail(tr("поверхность не поддерживается адаптером")))?;
    // не-sRGB формат: цвета и сглаживание выглядят как в привычных терминалах
    config.format = config.format.remove_srgb_suffix();
    // без этого режима композитор игнорирует прозрачность
    let modes = surface.get_capabilities(adapter).alpha_modes;
    if modes.contains(&wgpu::CompositeAlphaMode::PreMultiplied) {
        config.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;
    }
    // минимальная задержка ввода
    config.desired_maximum_frame_latency = 1;
    Ok(config)
}

fn instance_buffer(device: &wgpu::Device, quads: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("quads"),
        size: (quads * size_of::<Quad>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
