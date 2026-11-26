//! Цветовая тема и встроенные наборы.

/// Цвета терминала как 0xRRGGBB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub foreground: u32,
    pub background: u32,
    pub cursor: u32,
    pub selection: u32,
    /// Цвет выделенного текста; `None` — не менять.
    pub selection_text: Option<u32>,
    /// 8 обычных и 8 ярких цветов ANSI.
    pub ansi: [u32; 16],
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            foreground: 0xcdd6f4,
            background: 0x1e1e2e,
            cursor: 0xcba6f7,
            selection: 0xf5e0dc,
            selection_text: Some(0x1e1e2e),
            ansi: [
                0x45475a, 0xf38ba8, 0xa6e3a1, 0xf9e2af, 0x89b4fa, 0xf5c2e7, 0x94e2d5, 0xbac2de,
                0x585b70, 0xf7aec2, 0xc2ecbf, 0xfcd682, 0xaeccfc, 0xf398da, 0xb1eae1, 0xa6adc8,
            ],
        }
    }
}

// встроенные темы — такие же Lua-файлы, какие пишет пользователь
const BUILTIN: [(&str, &str); 3] = [
    (
        "catppuccin-mocha",
        include_str!("../themes/catppuccin-mocha.lua"),
    ),
    ("gruvbox-dark", include_str!("../themes/gruvbox-dark.lua")),
    ("nord", include_str!("../themes/nord.lua")),
];

pub(crate) fn builtin_source(name: &str) -> Option<&'static str> {
    BUILTIN
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, source)| *source)
}

pub fn builtin_names() -> impl Iterator<Item = &'static str> {
    BUILTIN.iter().map(|(name, _)| *name)
}
