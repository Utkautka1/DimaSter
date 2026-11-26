//! Проверка на сгенерированных файлах со всеми фильтрами и видами блоков.

use dimaster_image::decode_png;
use proptest::prelude::*;

fn expected(x: usize, y: usize) -> [u8; 4] {
    [
        (x * 4) as u8,
        (y * 7) as u8,
        ((x + y) % 11 * 20) as u8,
        255 - y as u8,
    ]
}

#[test]
fn decodes_rgba_in_all_block_types() {
    for data in [
        &include_bytes!("fixtures/rgba_dynamic.png")[..],
        include_bytes!("fixtures/rgba_stored.png"),
        include_bytes!("fixtures/rgba_fixed.png"),
    ] {
        let image = decode_png(data).expect("корректный PNG");
        for (i, pixel) in image.pixels.chunks(4).enumerate() {
            assert_eq!(pixel, expected(i % image.width, i / image.width));
        }
    }
}

#[test]
fn decodes_low_depth_palette_with_transparency() {
    let image = decode_png(include_bytes!("fixtures/palette2.png")).unwrap();
    assert_eq!((image.width, image.height), (5, 2));
    assert_eq!(&image.pixels[..8], [255, 0, 0, 128, 0, 255, 0, 255]);
    assert_eq!(&image.pixels[20..24], [9, 9, 9, 255]);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3000))]

    /// Повреждённые файлы дают `None`, а не панику.
    #[test]
    fn corrupted_files_do_not_panic(at in 8usize..400, value in any::<u8>(), cut in 0usize..400) {
        let mut data = include_bytes!("fixtures/rgba_dynamic.png").to_vec();
        let index = at % data.len();
        data[index] = value;
        data.truncate(data.len().saturating_sub(cut % 100));
        let _ = decode_png(&data);
    }

    #[test]
    fn garbage_does_not_panic(body in proptest::collection::vec(any::<u8>(), 0..300)) {
        let mut data = b"\x89PNG\r\n\x1a\n".to_vec();
        data.extend(body);
        let _ = decode_png(&data);
    }
}
