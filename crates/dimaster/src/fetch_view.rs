//! Расстановка картинки, рамки и строк `dimaster fetch`.
//!
//! Картинка не двигает курсор, поэтому вывод идёт построчно: на первой строке
//! рамки печатается сама картинка, дальше текст и рамка встают по колонкам.

use std::io::{self, Write};

use dimaster_config::{Layout, PicturePosition, PictureStyle};
use unicode_width::UnicodeWidthChar;

use crate::icat;

// колонки (с единицы) и строки, где стоят рамка и текст
struct Plan {
    width: usize,
    height: usize,
    // толщина рамки в ячейках: 0 или 1
    frame: usize,
    picture_column: usize,
    text_column: usize,
    // с какой строки начинается текст
    text_row: usize,
}

/// Ширина строки на экране без управляющих последовательностей цвета.
fn visible_width(text: &str) -> usize {
    let mut escaped = false;
    let shown = text.chars().filter(|&c| {
        let hidden = escaped || c == '\x1b';
        escaped = hidden && c != 'm';
        !hidden
    });
    shown.map(|c| c.width().unwrap_or(0)).sum()
}

// (колонка картинки, колонка текста, строка начала текста)
fn columns(layout: &Layout, outer: usize, height: usize) -> (usize, usize, usize) {
    let style = &layout.picture;
    let text = layout
        .lines
        .iter()
        .map(|l| visible_width(l))
        .max()
        .unwrap_or(0);
    let left = style.left + 1;
    let framed = height + usize::from(style.border.is_some()) * 2;
    match style.position {
        PicturePosition::Left => (left, left + outer + style.gap, style.top),
        PicturePosition::Right => (left + text + style.gap, left, style.top),
        PicturePosition::Top => (left, left, style.top + framed + 1),
    }
}

fn plan(layout: &Layout) -> Plan {
    let style = &layout.picture;
    let width = style
        .width
        .unwrap_or(layout.logo_width.saturating_sub(2))
        .max(1);
    let height = style.height.unwrap_or(width.div_ceil(2));
    let frame = usize::from(style.border.is_some());
    let (picture_column, text_column, text_row) = columns(layout, width + frame * 2, height);
    Plan {
        width,
        height,
        frame,
        picture_column,
        text_column,
        text_row,
    }
}

// кусок рамки в строке `row` блока картинки
fn border(out: &mut impl Write, plan: &Plan, style: &PictureStyle, row: usize) -> io::Result<()> {
    let Some([tl, tr, bl, br, horizontal, vertical]) = style.border else {
        return Ok(());
    };
    let color = dimaster_config::fetch_color(&style.border_color);
    let column = plan.picture_column;
    let last = plan.height + 1;
    let line = horizontal.to_string().repeat(plan.width);
    match row {
        0 => write!(out, "\x1b[{column}G\x1b[{color}m{tl}{line}{tr}\x1b[0m"),
        r if r == last => write!(out, "\x1b[{column}G\x1b[{color}m{bl}{line}{br}\x1b[0m"),
        _ => {
            let right = column + plan.width + 1;
            write!(
                out,
                "\x1b[{column}G\x1b[{color}m{vertical}\x1b[{right}G{vertical}\x1b[0m"
            )
        }
    }
}

fn row(
    out: &mut impl Write,
    layout: &Layout,
    plan: &Plan,
    data: &[u8],
    index: usize,
) -> io::Result<()> {
    let style = &layout.picture;
    let inside = index.checked_sub(style.top);
    if let Some(row) = inside.filter(|&r| r < plan.height + plan.frame * 2) {
        border(out, plan, style, row)?;
        if row == plan.frame {
            write!(out, "\x1b[{}G", plan.picture_column + plan.frame)?;
            let options = format!("c={},r={},fit={},C=1", plan.width, plan.height, style.fit);
            icat::show(out, data, &options)?;
        }
    }
    let text = index
        .checked_sub(plan.text_row)
        .and_then(|i| layout.lines.get(i));
    if let Some(text) = text {
        write!(out, "\x1b[{}G{text}", plan.text_column)?;
    }
    writeln!(out)
}

/// Печатает сведения вместе с картинкой.
pub(crate) fn with_picture(out: &mut impl Write, layout: &Layout, data: &[u8]) -> io::Result<()> {
    let plan = plan(layout);
    let picture = layout.picture.top + plan.height + plan.frame * 2;
    let rows = picture.max(plan.text_row + layout.lines.len());
    (0..rows).try_for_each(|index| row(out, layout, &plan, data, index))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(position: PicturePosition) -> Layout {
        let picture = PictureStyle {
            width: Some(10),
            border: Some(['a', 'b', 'c', 'd', '-', '|']),
            position,
            ..PictureStyle::default()
        };
        Layout {
            lines: vec!["\x1b[31mhello\x1b[0m".into()],
            picture,
            ..Layout::default()
        }
    }

    #[test]
    fn positions_move_picture_and_text() {
        assert_eq!(visible_width("\x1b[1m\x1b[31mпривет\x1b[0m"), 6);
        let left = plan(&layout(PicturePosition::Left));
        assert_eq!(
            (left.height, left.picture_column, left.text_column),
            (5, 1, 16)
        );
        let right = plan(&layout(PicturePosition::Right));
        assert_eq!((right.picture_column, right.text_column), (9, 1));
        assert_eq!(plan(&layout(PicturePosition::Top)).text_row, 8);
    }

    #[test]
    fn border_surrounds_the_picture_once() {
        let mut out = Vec::new();
        with_picture(&mut out, &layout(PicturePosition::Left), b"x").unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.matches("a----------b").count(), 1);
        assert_eq!(text.matches("fit=cover").count(), 1);
        assert_eq!(text.lines().count(), 7);
    }
}
