//! Перенос настроек из других терминалов.
//!
//! Берётся только то, что DimaSter умеет: шрифт, цвета, отступы, курсор,
//! история и shell. Незнакомые параметры молча пропускаются.

mod alacritty;
mod ghostty;
mod kitty;

use std::path::{Path, PathBuf};

use crate::Config;

// вложенные include глубже этого — почти наверняка цикл
const MAX_DEPTH: u8 = 4;

/// Терминал, из которого можно взять настройки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Ghostty,
    Kitty,
    Alacritty,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Self::Ghostty => "Ghostty",
            Self::Kitty => "kitty",
            Self::Alacritty => "Alacritty",
        }
    }

    fn file(self, config_home: &Path) -> PathBuf {
        config_home.join(match self {
            Self::Ghostty => "ghostty/config",
            Self::Kitty => "kitty/kitty.conf",
            Self::Alacritty => "alacritty/alacritty.toml",
        })
    }

    /// Накладывает настройки источника поверх `config`.
    pub fn apply(self, config_home: &Path, config: &mut Config) -> std::io::Result<()> {
        let path = self.file(config_home);
        let text = std::fs::read_to_string(&path)?;
        let directory = path.parent().unwrap_or(config_home);
        match self {
            Self::Ghostty => ghostty::apply(&text, directory, config, MAX_DEPTH),
            Self::Kitty => kitty::apply(&text, directory, config, MAX_DEPTH),
            Self::Alacritty => alacritty::apply(&text, config),
        }
        Ok(())
    }
}

/// Терминалы, у которых найден файл настроек.
pub fn detect(config_home: &Path) -> Vec<Source> {
    [Source::Ghostty, Source::Kitty, Source::Alacritty]
        .into_iter()
        .filter(|s| s.file(config_home).is_file())
        .collect()
}

// `~/x` и относительные пути из include
fn resolve(directory: &Path, value: &str) -> PathBuf {
    match (value.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => directory.join(value),
    }
}

/// Тема Ghostty по имени: пользовательская или установленная с Ghostty.
pub(crate) fn ghostty_theme(name: &str, config_home: Option<&Path>) -> Option<crate::Theme> {
    // имя — это файл в каталоге тем, а не путь
    if name.contains(['/', '\\']) || name.starts_with('.') {
        return None;
    }
    let own = config_home.map(|home| home.join("ghostty/themes").join(name));
    let system = Path::new("/usr/share/ghostty/themes").join(name);
    let text = own
        .into_iter()
        .chain([system])
        .find_map(|p| std::fs::read_to_string(p).ok())?;
    let mut config = Config::default();
    ghostty::apply(&text, Path::new("/"), &mut config, 0);
    Some(config.theme)
}
