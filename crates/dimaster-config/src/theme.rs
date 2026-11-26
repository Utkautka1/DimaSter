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
            foreground: 0xd8dee9,
            background: 0x11131a,
            cursor: 0xe5e9f0,
            selection: 0x3b4261,
            selection_text: None,
            ansi: [
                0x1d1f28, 0xf7768e, 0x9ece6a, 0xe0af68, 0x7aa2f7, 0xbb9af7, 0x7dcfff, 0xc0caf5,
                0x565f89, 0xff899d, 0xb9f27c, 0xffc777, 0x8db0ff, 0xc7a9ff, 0xa4daff, 0xe5e9f0,
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
