//! Текстуры картинок: загружаются при первом показе, живут пока видимы.

use std::collections::{HashMap, HashSet};

use dimaster_grid::Picture;

use crate::images::ImageQuad;
use crate::pipeline::Pipeline;

/// Привязки по номеру картинки и номеру кадра.
#[derive(Default)]
pub(crate) struct Textures(HashMap<(u64, usize), wgpu::BindGroup>);

impl Textures {
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipeline: &Pipeline,
        images: &[ImageQuad],
    ) {
        let visible: HashSet<u64> = images.iter().map(|i| i.picture.serial).collect();
        self.0.retain(|(serial, _), _| visible.contains(serial));
        for image in images {
            let key = (image.picture.serial, image.frame);
            if self.0.contains_key(&key) {
                continue;
            }
            if let Some(texture) = upload(device, queue, &image.picture, image.frame) {
                self.0.insert(key, pipeline.bind(device, &texture));
            }
        }
    }

    pub(crate) fn get(&self, image: &ImageQuad) -> Option<&wgpu::BindGroup> {
        self.0.get(&(image.picture.serial, image.frame))
    }
}

// картинка больше предела видеокарты не показывается
fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    picture: &Picture,
    frame: usize,
) -> Option<wgpu::Texture> {
    let size = extent(device, picture)?;
    let texture = crate::pipeline::create_texture(device, size);
    let layout = wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(size.width * 4),
        rows_per_image: None,
    };
    let pixels = &picture.animation.frames[frame].pixels;
    queue.write_texture(texture.as_image_copy(), pixels, layout, size);
    Some(texture)
}

fn extent(device: &wgpu::Device, picture: &Picture) -> Option<wgpu::Extent3d> {
    let (width, height) = (picture.animation.width, picture.animation.height);
    let limit = device.limits().max_texture_dimension_2d as usize;
    if width > limit || height > limit {
        log::warn!("Картинка {width}x{height} слишком велика");
        return None;
    }
    Some(wgpu::Extent3d {
        width: width as u32,
        height: height as u32,
        depth_or_array_layers: 1,
    })
}
