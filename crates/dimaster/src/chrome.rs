//! Своя рамка: полоса заголовка, кнопки и изменение размера за края.

use dimaster_config::Decorations;
use winit::window::{CursorIcon, ResizeDirection};

use crate::view::View;

// ширина зоны захвата края окна в пикселях
const EDGE: f64 = 6.0;
// ширина одной кнопки в ячейках
const BUTTON: u32 = 3;

pub(crate) const BUTTONS: &str = " —  □  ✕ ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Button {
    Minimize,
    Maximize,
    Close,
}

/// Край или угол окна под точкой.
pub(crate) fn edge(x: f64, y: f64, width: f64, height: f64) -> Option<ResizeDirection> {
    use ResizeDirection as Dir;
    let (west, east) = (x < EDGE, x > width - EDGE);
    let (north, south) = (y < EDGE, y > height - EDGE);
    Some(match (north, south, west, east) {
        (true, _, true, _) => Dir::NorthWest,
        (true, _, _, true) => Dir::NorthEast,
        (_, true, true, _) => Dir::SouthWest,
        (_, true, _, true) => Dir::SouthEast,
        (true, ..) => Dir::North,
        (_, true, ..) => Dir::South,
        (_, _, true, _) => Dir::West,
        (_, _, _, true) => Dir::East,
        _ => return None,
    })
}

/// Кнопка заголовка под горизонтальной координатой.
pub(crate) fn button(x: f64, width: u32, cell: u32) -> Option<Button> {
    let from_right = f64::from(width) - x;
    let index = (from_right / f64::from(BUTTON * cell)).floor() as i64;
    match index {
        0 => Some(Button::Close),
        1 => Some(Button::Maximize),
        2 => Some(Button::Minimize),
        _ => None,
    }
}

impl View {
    /// Нажатие левой кнопки по рамке; `true` — до терминала оно не доходит.
    pub(crate) fn chrome_press(&mut self) -> bool {
        let (x, y) = self.pointer.position;
        let size = self.window.inner_size();
        if self.config.window.decorations == Decorations::System {
            return false;
        }
        if let Some(direction) = edge(x, y, size.width.into(), size.height.into()) {
            let _ = self.window.drag_resize_window(direction);
            return true;
        }
        if y >= f64::from(self.title_height()) {
            return false;
        }
        match button(x, size.width, self.fonts.metrics().cell_width) {
            Some(Button::Close) => self.closing = true,
            Some(Button::Maximize) => self.window.set_maximized(!self.window.is_maximized()),
            Some(Button::Minimize) => self.window.set_minimized(true),
            None => {
                let _ = self.window.drag_window();
            }
        }
        true
    }

    // стрелки изменения размера у краёв окна без системной рамки
    pub(crate) fn update_pointer_icon(&self) {
        let (x, y) = self.pointer.position;
        let size = self.window.inner_size();
        let resize = edge(x, y, size.width.into(), size.height.into())
            .filter(|_| self.config.window.decorations != Decorations::System);
        let icon = match resize {
            Some(direction) => CursorIcon::from(direction),
            None if self.hover_link.is_some() => CursorIcon::Pointer,
            None => CursorIcon::Text,
        };
        self.window.set_cursor(icon);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_testing() {
        assert_eq!(
            edge(1.0, 1.0, 800.0, 600.0),
            Some(ResizeDirection::NorthWest)
        );
        assert_eq!(
            edge(400.0, 599.0, 800.0, 600.0),
            Some(ResizeDirection::South)
        );
        assert_eq!(edge(400.0, 300.0, 800.0, 600.0), None);
        assert_eq!(button(790.0, 800, 10), Some(Button::Close));
        assert_eq!(button(730.0, 800, 10), Some(Button::Minimize));
        assert_eq!(button(100.0, 800, 10), None);
    }
}
