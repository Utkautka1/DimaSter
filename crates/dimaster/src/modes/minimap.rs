//! Режим миникарты: перемещение по истории с обзором всего буфера.

use super::Press;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Minimap {
    // прокрутка, к которой возвращает Esc
    pub(crate) origin: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Новое положение прокрутки, режим продолжается.
    Scroll(usize),
    /// Остаться в выбранном месте.
    Accept,
    Cancel,
}

impl Minimap {
    /// `scroll` — строк назад от конца, `max` — вся история, `rows` — высота экрана.
    pub(crate) fn press(self, press: Press, scroll: usize, max: usize, rows: usize) -> Outcome {
        let step = (rows / 2).max(1);
        let next = match press {
            Press::Escape | Press::Char('q') => return Outcome::Cancel,
            Press::Enter { .. } => return Outcome::Accept,
            Press::Up | Press::Char('k') => scroll + step,
            Press::Down | Press::Char('j') => scroll.saturating_sub(step),
            Press::Char('g') => max,
            Press::Char('G') => 0,
            _ => scroll,
        };
        Outcome::Scroll(next.min(max))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_by_half_screen_within_history() {
        let map = Minimap { origin: 0 };
        assert_eq!(map.press(Press::Char('k'), 0, 100, 20), Outcome::Scroll(10));
        assert_eq!(
            map.press(Press::Char('k'), 95, 100, 20),
            Outcome::Scroll(100)
        );
        assert_eq!(map.press(Press::Down, 5, 100, 20), Outcome::Scroll(0));
        assert_eq!(map.press(Press::Escape, 5, 100, 20), Outcome::Cancel);
    }
}
