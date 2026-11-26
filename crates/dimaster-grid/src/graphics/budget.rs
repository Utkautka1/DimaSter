//! Общий лимит памяти на показанные картинки.
//!
//! Безымянные картинки не попадают в хранилище, но живут в строках экрана и
//! истории: без лимита поток Sixel занял бы всю память.

use std::collections::BTreeMap;

use super::Placement;
use crate::terminal::State;

// суммарный объём пикселей всех различных картинок
const LIMIT: usize = 256 * 1024 * 1024;

// номера растут со временем, поэтому порядок ключей — порядок появления
fn doomed<'a>(placements: impl Iterator<Item = &'a Placement>) -> Option<u64> {
    let sizes: BTreeMap<u64, usize> = placements
        .map(|p| (p.picture.serial, p.picture.bytes()))
        .collect();
    let mut excess = sizes
        .values()
        .sum::<usize>()
        .checked_sub(LIMIT)
        .filter(|e| *e > 0)?;
    sizes.into_iter().find_map(|(serial, bytes)| {
        excess = excess.saturating_sub(bytes);
        (excess == 0).then_some(serial)
    })
}

impl State {
    /// Убирает самые старые картинки, пока объём не уложится в лимит.
    pub(crate) fn enforce_image_budget(&mut self) {
        let screens = [&self.screen, &self.other];
        let images = screens.into_iter().flat_map(|s| {
            s.history
                .iter()
                .flat_map(|r| &r.images)
                .chain(s.rows.iter().flat_map(|r| &r.images))
        });
        let Some(oldest_kept) = doomed(images) else {
            return;
        };
        for screen in [&mut self.screen, &mut self.other] {
            for row in screen.history.iter_mut() {
                row.images.retain(|p| p.picture.serial > oldest_kept);
            }
            for row in &mut screen.rows {
                row.images.retain(|p| p.picture.serial > oldest_kept);
            }
        }
    }
}
