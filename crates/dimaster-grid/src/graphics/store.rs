//! Переданные, но ещё не обязательно показанные картинки Kitty.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use dimaster_image::Animation;

use super::{Pending, Picture};

// суммарный объём хранимых пикселей
const LIMIT: usize = 256 * 1024 * 1024;

#[derive(Debug, Default)]
pub(crate) struct Store {
    images: HashMap<u32, Arc<Picture>>,
    serial: u64,
    /// Незаконченная передача по частям.
    pub(crate) pending: Option<Pending>,
}

impl Store {
    pub(crate) fn picture(&mut self, animation: Animation) -> Arc<Picture> {
        self.serial += 1;
        Arc::new(Picture {
            serial: self.serial,
            animation,
            started: Instant::now(),
        })
    }

    /// Самые старые картинки вытесняются, когда память заканчивается.
    pub(crate) fn keep(&mut self, id: u32, picture: Arc<Picture>) {
        self.images.insert(id, picture);
        while self
            .images
            .values()
            .map(|p| p.animation.bytes())
            .sum::<usize>()
            > LIMIT
        {
            let oldest = self
                .images
                .iter()
                .min_by_key(|(_, p)| p.serial)
                .map(|(&id, _)| id);
            let Some(oldest) = oldest else { return };
            self.images.remove(&oldest);
        }
    }

    pub(crate) fn get(&self, id: u32) -> Option<Arc<Picture>> {
        self.images.get(&id).cloned()
    }

    pub(crate) fn remove(&mut self, id: Option<u32>) {
        match id {
            Some(id) => {
                self.images.remove(&id);
            }
            None => self.images.clear(),
        }
    }
}
