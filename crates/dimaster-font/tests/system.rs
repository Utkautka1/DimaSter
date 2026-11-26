//! Проверка на настоящих системных шрифтах.

use dimaster_font::{Bitmap, FontSystem};

fn fonts() -> FontSystem {
    FontSystem::new(16.0).expect("в системе есть моноширинный шрифт")
}

fn first_bitmap(fonts: &mut FontSystem, text: &str, cells: u32) -> Bitmap {
    let glyphs = fonts.shape(text, false, false);
    fonts.glyph(glyphs[0].id, cells).clone()
}

#[test]
fn renders_latin_and_cyrillic() {
    let mut fonts = fonts();
    let metrics = fonts.metrics();
    assert!(metrics.cell_width > 0 && metrics.cell_height > metrics.cell_width);
    for text in ["A", "ё"] {
        assert!(
            first_bitmap(&mut fonts, text, 1).data.contains(&255),
            "пустой глиф {text}"
        );
    }
    assert!(first_bitmap(&mut fonts, " ", 1).data.is_empty());
}

#[test]
fn shaping_keeps_clusters_per_character() {
    let glyphs = fonts().shape("aё漢b", false, false);
    let clusters: Vec<_> = glyphs.iter().map(|g| g.cluster).collect();
    assert_eq!(clusters, [0, 1, 3, 6]);
}

#[test]
fn emoji_is_colored_and_fits_two_cells() {
    let mut fonts = fonts();
    let bitmap = first_bitmap(&mut fonts, "🚀", 2);
    // без цветного эмодзи-шрифта в системе проверять нечего
    if !bitmap.color {
        return;
    }
    let metrics = fonts.metrics();
    assert!(bitmap.width <= 2 * metrics.cell_width as usize);
    assert!(bitmap.height <= metrics.cell_height as usize);
    assert_eq!(bitmap.data.len(), bitmap.width * bitmap.height * 4);
}

#[test]
fn symbol_emoji_use_color_font() {
    let mut fonts = fonts();
    if !first_bitmap(&mut fonts, "🚀", 2).color {
        return;
    }
    assert!(first_bitmap(&mut fonts, "✅", 2).color);
    assert!(first_bitmap(&mut fonts, "❤\u{fe0f}", 1).color);
    assert!(!first_bitmap(&mut fonts, "❤", 1).color);
}
