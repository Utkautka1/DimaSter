//! Проверка на настоящих системных шрифтах.

use dimaster_font::{FontSystem, GlyphKey};

#[test]
fn renders_latin_and_cyrillic() {
    let mut fonts = FontSystem::new(16.0).expect("в системе есть моноширинный шрифт");
    let metrics = fonts.metrics();
    assert!(metrics.cell_width > 0 && metrics.cell_height > metrics.cell_width);
    for ch in ['A', 'ё'] {
        let bitmap = fonts.glyph(GlyphKey {
            ch,
            bold: false,
            italic: false,
        });
        assert!(bitmap.coverage.contains(&255), "пустой глиф {ch}");
    }
    assert!(
        fonts
            .glyph(GlyphKey {
                ch: ' ',
                bold: false,
                italic: false
            })
            .coverage
            .is_empty()
    );
}
