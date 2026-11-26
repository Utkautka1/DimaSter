//! Цвета в виде `#rrggbb`.

/// Принимает `#rrggbb`, `rrggbb` и `#rgb`; результат — 0xRRGGBB.
pub fn parse_color(text: &str) -> Option<u32> {
    let hex = text.trim().trim_start_matches('#');
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match hex.len() {
        6 => u32::from_str_radix(hex, 16).ok(),
        3 => {
            let doubled: String = hex.chars().flat_map(|c| [c, c]).collect();
            u32::from_str_radix(&doubled, 16).ok()
        }
        _ => None,
    }
}

pub(crate) fn format_color(color: u32) -> String {
    format!("#{color:06x}")
}
