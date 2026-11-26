//! Конвейер wgpu и ресурсы, которые он читает.

use crate::atlas::{ATLAS_SIZE, Upload};
use crate::frame::Quad;

/// Конвейер, атлас и uniform с размером окна.
pub(crate) struct Pipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) bindings: wgpu::BindGroup,
    pub(crate) screen: wgpu::Buffer,
    sampler: wgpu::Sampler,
    atlas: wgpu::Texture,
}

impl Pipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let pipeline = create_pipeline(device, format);
        let atlas_size = wgpu::Extent3d {
            width: ATLAS_SIZE,
            height: ATLAS_SIZE,
            depth_or_array_layers: 1,
        };
        let atlas = create_texture(device, atlas_size);
        let (screen, sampler) = (create_uniform(device), create_sampler(device));
        let bindings = bind(device, &pipeline, &screen, &sampler, &atlas);
        Self {
            pipeline,
            bindings,
            screen,
            sampler,
            atlas,
        }
    }

    /// Привязки с другой текстурой — для картинок.
    pub(crate) fn bind(&self, device: &wgpu::Device, texture: &wgpu::Texture) -> wgpu::BindGroup {
        bind(device, &self.pipeline, &self.screen, &self.sampler, texture)
    }

    pub(crate) fn upload(&self, queue: &wgpu::Queue, upload: &Upload) {
        let slot = upload.slot;
        let target = wgpu::TexelCopyTextureInfo {
            texture: &self.atlas,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: slot.x,
                y: slot.y,
                z: 0,
            },
            aspect: wgpu::TextureAspect::All,
        };
        let layout = wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(slot.width * 4),
            rows_per_image: None,
        };
        let size = wgpu::Extent3d {
            width: slot.width,
            height: slot.height,
            depth_or_array_layers: 1,
        };
        queue.write_texture(target, &upload.data, layout, size);
    }
}

// размер окна для перевода пикселей в координаты видеокарты
fn create_uniform(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("screen"),
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

// картинки масштабируются под ячейки, поэтому нужна билинейная фильтрация
fn create_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}

pub(crate) fn create_texture(device: &wgpu::Device, size: wgpu::Extent3d) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn bind(
    device: &wgpu::Device,
    pipeline: &wgpu::RenderPipeline,
    screen: &wgpu::Buffer,
    sampler: &wgpu::Sampler,
    texture: &wgpu::Texture,
) -> wgpu::BindGroup {
    let view = texture.create_view(&Default::default());
    let texture = wgpu::BindingResource::TextureView(&view);
    let sampler = wgpu::BindingResource::Sampler(sampler);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bindings"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: screen.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: texture,
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: sampler,
            },
        ],
    })
}

fn create_pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
    let attributes = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x2,
        3 => Uint32,
        4 => Float32x2
    ];
    let buffers = [Some(wgpu::VertexBufferLayout {
        array_stride: size_of::<Quad>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &attributes,
    })];
    let targets = [Some(wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })];
    build_pipeline(device, &module, &buffers, &targets)
}

fn build_pipeline(
    device: &wgpu::Device,
    module: &wgpu::ShaderModule,
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
    targets: &[Option<wgpu::ColorTargetState>],
) -> wgpu::RenderPipeline {
    let topology = wgpu::PrimitiveTopology::TriangleStrip;
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("quads"),
        layout: None,
        vertex: vertex_stage(module, buffers),
        primitive: wgpu::PrimitiveState {
            topology,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(fragment_stage(module, targets)),
        multiview_mask: None,
        cache: None,
    })
}

fn vertex_stage<'a>(
    module: &'a wgpu::ShaderModule,
    buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
) -> wgpu::VertexState<'a> {
    let compilation_options = Default::default();
    wgpu::VertexState {
        module,
        entry_point: Some("vertex"),
        compilation_options,
        buffers,
    }
}

fn fragment_stage<'a>(
    module: &'a wgpu::ShaderModule,
    targets: &'a [Option<wgpu::ColorTargetState>],
) -> wgpu::FragmentState<'a> {
    let compilation_options = Default::default();
    wgpu::FragmentState {
        module,
        entry_point: Some("fragment"),
        compilation_options,
        targets,
    }
}
