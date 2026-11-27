//! Декодирование картинок для графических протоколов.
//!
//! PNG, GIF и Sixel разбираются своим кодом. JPEG, WebP, SVG и прочие
//! сложные форматы отдаются `ffmpeg` — писать эти кодеки самим нереалистично.

mod animation;
mod base64;
mod external;
mod gif;
mod inflate;
mod png;
mod sixel;

pub use animation::{Animation, Decoding, Frame, decode, decode_external, decode_native};
pub use base64::{decode_base64, encode_base64};
pub use gif::decode_gif;
pub use png::{Image, decode_png};
pub use sixel::decode_sixel;

/// Распаковывает zlib-поток не длиннее `limit` байт.
pub fn inflate_zlib(data: &[u8], limit: usize) -> Option<Vec<u8>> {
    inflate::zlib(data, limit)
}
