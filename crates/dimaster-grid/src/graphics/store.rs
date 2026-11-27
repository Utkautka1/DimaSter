//! Переданные, но ещё не обязательно показанные картинки Kitty.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use dimaster_image::Animation;

use super::{Pending, Picture, VirtualPlacement};

// суммарный объём хранимых пикселей; он же предел кадров одной анимации
pub(super) const LIMIT: usize = 256 * 1024 * 1024;

#[derive(Debug, Default)]
pub(crate) struct Store {
    images: HashMap<u32, Arc<Picture>>,
    virtuals: HashMap<(u32, u32), VirtualPlacement>,
    serial: u64,
    /// Незаконченная передача по частям.
    pub(crate) pending: Option<Pending>,
}

impl Store {
    pub(crate) fn picture(&mut self, animation: Animation) -> Arc<Picture> {
        self.serial += 1;
        Arc::new(Picture::new(self.serial, animation))
    }

    pub(crate) fn keep(&mut self, id: u32, picture: Arc<Picture>) {
        self.images.insert(id, picture);
        self.trim();
    }

    /// Вытесняет самые старые картинки, пока всё хранимое не уложится в лимит.
    pub(crate) fn trim(&mut self) {
        // одна картинка может быть и сохранённой, и виртуальной: считаем раз
        let virtuals = self.virtuals.values().map(|v| &v.picture);
        let sizes: BTreeMap<u64, usize> = self
            .images
            .values()
            .chain(virtuals)
            .map(|p| (p.serial, p.bytes()))
            .collect();
        let mut total: usize = sizes.values().sum();
        for (serial, bytes) in sizes {
            if total <= LIMIT {
                return;
            }
            self.forget(serial);
            total -= bytes;
        }
    }

    fn forget(&mut self, serial: u64) {
        self.images.retain(|_, p| p.serial != serial);
        self.virtuals.retain(|_, v| v.picture.serial != serial);
    }

    pub(crate) fn get(&self, id: u32) -> Option<Arc<Picture>> {
        self.images.get(&id).cloned()
    }

    pub(crate) fn put_virtual(&mut self, id: (u32, u32), placement: VirtualPlacement) {
        self.virtuals.insert(id, placement);
        self.trim();
    }

    pub(crate) fn get_virtual(
        &self,
        image_id: u32,
        placement_id: u32,
    ) -> Option<&VirtualPlacement> {
        if let Some(found) = self.virtuals.get(&(image_id, placement_id)) {
            return Some(found);
        }
        (placement_id == 0).then(|| {
            self.virtuals
                .iter()
                .find(|((img, _), _)| *img == image_id)
                .map(|(_, vp)| vp)
        })?
    }

    pub(crate) fn remove(&mut self, id: Option<u32>) {
        match id {
            Some(id) => {
                self.images.remove(&id);
                self.virtuals.retain(|(img, _), _| *img != id);
            }
            None => {
                self.images.clear();
                self.virtuals.clear();
            }
        }
    }

    pub(crate) fn remove_virtual(&mut self, image_id: u32, placement_id: u32) {
        self.virtuals.retain(|&(img, pl), _| {
            !(img == image_id && (placement_id == 0 || pl == placement_id))
        });
    }
}
