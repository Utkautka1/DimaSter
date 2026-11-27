//! Свои фоны витрины из `~/.local/share/dimaster/backgrounds/`.
//!
//! Каждая картинка — отдельный стиль: цвета берутся из неё самой, а
//! расположение элементов — у встроенного стиля, названного в файле
//! `<имя>.style` рядом (без него — «Стекло»). Список читается при запуске,
//! сама картинка — только когда её выбрали.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use dimaster_config::{Style, Theme, photo_theme, style, styles};
use dimaster_grid::Picture;
use dimaster_i18n::t;
use dimaster_image::{Animation, decode};

use crate::history_store::data_dir;
use crate::settings;

/// Декодированный фон и всё, что из него выведено.
pub(crate) struct Loaded {
    pub(crate) theme: Theme,
    pub(crate) picture: Arc<Picture>,
    /// Непрозрачность картинки за текстом: чем светлее, тем меньше.
    pub(crate) shade: f32,
}

pub(crate) struct Photo {
    pub(crate) name: String,
    path: PathBuf,
    pub(crate) base: &'static Style,
    loaded: OnceLock<Option<Loaded>>,
}

impl Photo {
    /// Картинка с диска; `None`, если её не удалось прочитать.
    pub(crate) fn loaded(&self) -> Option<&Loaded> {
        self.loaded.get_or_init(|| load(&self.path)).as_ref()
    }
}

fn load(path: &Path) -> Option<Loaded> {
    let animation = std::fs::read(path).ok().and_then(|data| decode(&data));
    let Some(animation) = animation else {
        let error = path.display();
        log::warn!("{}", t!("Фон не прочитан: {path}", path = error));
        return None;
    };
    Some(prepare(animation))
}

fn prepare(animation: Animation) -> Loaded {
    let pixels = animation.frames.first().map(|f| f.pixels.as_slice());
    let (theme, light) = photo_theme(pixels.unwrap_or_default());
    Loaded {
        theme,
        picture: settings::picture(animation),
        shade: (0.62 - light * 0.5)
            .max(dimaster_limits::PHOTO_SHADE_MIN.u32() as f32 / 100.0)
            .min(dimaster_limits::PHOTO_SHADE_MAX.u32() as f32 / 100.0),
    }
}

fn photo(path: PathBuf) -> Option<Photo> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    if !["png", "jpg", "jpeg", "gif", "webp"].contains(&extension.as_str()) {
        return None;
    }
    let chosen = std::fs::read_to_string(path.with_extension("style")).ok();
    let base = chosen.and_then(|id| style(id.trim()));
    Some(Photo {
        name: path.file_stem()?.to_string_lossy().into_owned(),
        base: base.or_else(|| styles().first())?,
        path,
        loaded: OnceLock::new(),
    })
}

fn scan(directory: &Path) -> Vec<Photo> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let paths = entries.flatten().map(|e| e.path());
    let mut found: Vec<Photo> = paths.filter_map(photo).collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// Все свои фоны по алфавиту.
pub(crate) fn photos() -> &'static [Photo] {
    static PHOTOS: OnceLock<Vec<Photo>> = OnceLock::new();
    PHOTOS.get_or_init(|| data_dir().map_or_else(Vec::new, |d| scan(&d.join("backgrounds"))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_gives_sorted_styles() {
        let directory =
            std::env::temp_dir().join(format!("dimaster-photos-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("каталог");
        for (name, text) in [
            ("b.jpg", ""),
            ("a.png", ""),
            ("a.style", "neon\n"),
            ("note.txt", ""),
        ] {
            std::fs::write(directory.join(name), text).expect("файл");
        }
        let found = scan(&directory);
        std::fs::remove_dir_all(&directory).expect("уборка");
        let list: Vec<_> = found.iter().map(|p| (p.name.as_str(), p.base.id)).collect();
        assert_eq!(list, [("a", "neon"), ("b", "glass")]);
    }
}
