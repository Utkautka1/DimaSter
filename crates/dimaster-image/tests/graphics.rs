//! GIF, Sixel, base64 и выбор кадра.

use std::time::Duration;

use dimaster_image::{decode, decode_base64, decode_gif, decode_sixel, encode_base64};
use proptest::prelude::*;

const GIF: &[u8] = include_bytes!("fixtures/anim.gif");

#[test]
fn gif_frames_colors_and_timing() {
    let animation = decode_gif(GIF).unwrap();
    assert_eq!(
        (animation.width, animation.height, animation.frames.len()),
        (64, 48, 2)
    );
    let [red, blue] = [
        &animation.frames[0].pixels[..4],
        &animation.frames[1].pixels[..4],
    ];
    assert!(red[0] > 200 && red[2] < 60, "первый кадр красный: {red:?}");
    // палитра ffmpeg дизерит синий между 170 и 255
    assert!(
        blue[2] >= 170 && blue[0] < 60,
        "второй кадр синий: {blue:?}"
    );
    let delay = u64::from(animation.frames[0].delay);
    assert_eq!(animation.frame_at(Duration::ZERO).0, 0);
    assert_eq!(animation.frame_at(Duration::from_millis(delay)).0, 1);
}

#[test]
fn sixel_colors_repeat_and_bands() {
    let image = decode_sixel(b"#1;2;100;0;0#1!3~-#2;2;0;100;0#2@").unwrap();
    assert_eq!((image.width, image.height), (3, 7));
    assert_eq!(&image.pixels[..4], [255, 0, 0, 255]);
    assert_eq!(&image.pixels[6 * 3 * 4..][..4], [0, 255, 0, 255]);
    assert_eq!(image.pixels[(6 * 3 + 1) * 4 + 3], 0);
}

#[test]
fn base64_round_trip() {
    for data in [&b""[..], b"a", b"ab", b"abc", b"\x00\xff\x10\x20"] {
        assert_eq!(decode_base64(encode_base64(data).as_bytes()).unwrap(), data);
    }
    assert_eq!(encode_base64(b"hi"), "aGk=");
}

#[test]
fn jpeg_goes_through_ffmpeg() {
    let image = decode(include_bytes!("fixtures/red.jpg")).expect("нужен ffmpeg");
    assert_eq!((image.width, image.height), (16, 8));
    assert!(image.frames[0].pixels[0] > 200);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(3000))]

    #[test]
    fn damaged_gif_and_sixel_do_not_panic(at in 0usize..4000, value in any::<u8>(), cut in 0usize..200) {
        let mut data = GIF.to_vec();
        let index = at % data.len();
        data[index] = value;
        data.truncate(data.len().saturating_sub(cut));
        let _ = decode_gif(&data);
        let _ = decode_sixel(&data);
    }

    #[test]
    fn random_sixel_does_not_panic(data in "[#!\"$\\-?-~0-9;]{0,64}") {
        let _ = decode_sixel(data.as_bytes());
    }
}

// повторы поверх одних и тех же пикселей не растят картинку, но тратят время
#[test]
fn sixel_overdraw_is_bounded_in_time() {
    let data = "!4096~$".repeat(200_000);
    let started = std::time::Instant::now();
    let _ = decode_sixel(data.as_bytes());
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
}
