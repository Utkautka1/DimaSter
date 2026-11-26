//! Base64: так передаются данные во всех графических протоколах.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn value(byte: u8) -> Option<u32> {
    Some(u32::from(match byte {
        b'A'..=b'Z' => byte - b'A',
        b'a'..=b'z' => byte - b'a' + 26,
        b'0'..=b'9' => byte - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    }))
}

/// Пробелы и переводы строк пропускаются, `=` завершает данные.
pub fn decode_base64(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() / 4 * 3);
    let (mut buffer, mut bits) = (0u32, 0);
    for &byte in data.iter().filter(|b| !b.is_ascii_whitespace()) {
        if byte == b'=' {
            break;
        }
        buffer = (buffer << 6) | value(byte)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

pub fn encode_base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let mut group = [0; 3];
        group[..chunk.len()].copy_from_slice(chunk);
        let number = u32::from_be_bytes([0, group[0], group[1], group[2]]);
        for index in 0..4 {
            if index <= chunk.len() {
                let six = (number >> (18 - index * 6)) & 63;
                out.push(char::from(ALPHABET[six as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}
