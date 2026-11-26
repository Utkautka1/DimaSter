//! Своя рамка: полоса заголовка, кнопки и изменение размера за края.

use dimaster_config::{Align, Decorations, TitleButton, TitleStyle, WindowButton};
use dimaster_render::Label;
use unicode_width::UnicodeWidthStr;
use winit::window::{CursorIcon, ResizeDirection};

use crate::overlay::{Drawn, clip, fill};
use crate::view::View;

// ширина зоны захвата края окна в пикселях
const EDGE: f64 = 6.0;
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

/// Место кнопки заголовка: левый край, ширина и сама кнопка.
type Slot<'a> = (u32, u32, &'a TitleButton);

/// Раскладывает кнопки подряд у выбранного края окна.
fn slots(style: &TitleStyle, window: u32, cell: u32) -> Vec<Slot<'_>> {
    let width = |button: &TitleButton| button.text.width() as u32 * cell;
    let total: u32 = style.buttons.iter().map(width).sum();
    let mut x = match style.buttons_left {
        true => 0,
        false => window.saturating_sub(total),
    };
    let place = |button| {
        let slot = (x, width(button), button);
        x += slot.1;
        slot
    };
    style.buttons.iter().map(place).collect()
}

fn button(x: f64, slots: &[Slot]) -> Option<WindowButton> {
    let inside = |s: &&Slot| x >= f64::from(s.0) && x < f64::from(s.0 + s.1);
    slots.iter().find(inside).map(|s| s.2.kind)
}

// левый край заголовка в свободной от кнопок части `free`
fn title_x(align: Align, window: u32, free: (u32, u32), text: u32) -> u32 {
    let last = free.1.saturating_sub(text).max(free.0);
    match align {
        Align::Left => free.0,
        Align::Center => (window.saturating_sub(text) / 2).clamp(free.0, last),
        Align::Right => last,
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
        let cell = self.fonts.metrics().cell_width;
        match button(x, &slots(&self.config.ui.title, size.width, cell)) {
            Some(WindowButton::Close) => self.closing = true,
            Some(WindowButton::Maximize) => self.window.set_maximized(!self.window.is_maximized()),
            Some(WindowButton::Minimize) => self.window.set_minimized(true),
            None => {
                let _ = self.window.drag_window();
            }
        }
        true
    }

    // часть полосы без кнопок, с отступом в ячейку
    fn free_space(&self, slots: &[Slot]) -> (u32, u32) {
        let cell = self.fonts.metrics().cell_width;
        let width = self.window.inner_size().width;
        let used: u32 = slots.iter().map(|s| s.1).sum();
        let (left, right) = match self.config.ui.title.buttons_left {
            true => (used, 0),
            false => (0, used),
        };
        (left + cell, width.saturating_sub(right + cell))
    }

    fn title_label(&self, x: u32, text: String, color: Option<u32>) -> Label {
        let (style, theme) = (&self.config.ui.title, self.config.theme);
        Label {
            x,
            y: style.padding / 2,
            width: 0,
            text,
            color: color.or(style.color).unwrap_or(theme.foreground),
            background: theme.background,
        }
    }

    fn caption(&self, slots: &[Slot]) -> Label {
        let style = &self.config.ui.title;
        let (cell, window) = (
            self.fonts.metrics().cell_width,
            self.window.inner_size().width,
        );
        let free = self.free_space(slots);
        let text = fill(&style.format, &[("title", &self.window.title())]);
        let text = clip(&text, (free.1.saturating_sub(free.0) / cell) as usize);
        let x = title_x(style.align, window, free, text.width() as u32 * cell);
        self.title_label(x, text, None)
    }

    pub(crate) fn title_bar(&self) -> Drawn {
        let mut out = Drawn::default();
        if self.title_height() == 0 {
            return out;
        }
        let (style, window) = (&self.config.ui.title, self.window.inner_size().width);
        if let Some(color) = style.background {
            let size = (window, self.title_height());
            out.rects.push(self.plain_rect(0, 0, size, color));
        }
        let slots = slots(style, window, self.fonts.metrics().cell_width);
        out.labels.push(self.caption(&slots));
        let buttons = slots
            .iter()
            .map(|s| self.title_label(s.0, s.2.text.clone(), s.2.color));
        out.labels.extend(buttons);
        out
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
    }

    #[test]
    fn buttons_follow_their_side() {
        let mut style = TitleStyle::default();
        let right = slots(&style, 800, 10);
        assert_eq!(button(790.0, &right), Some(WindowButton::Close));
        assert_eq!(button(730.0, &right), Some(WindowButton::Minimize));
        assert_eq!(button(100.0, &right), None);
        style.buttons_left = true;
        assert_eq!(
            button(5.0, &slots(&style, 800, 10)),
            Some(WindowButton::Minimize)
        );
        assert_eq!(title_x(Align::Center, 800, (100, 790), 100), 350);
        assert_eq!(title_x(Align::Right, 800, (100, 790), 100), 690);
    }
}
