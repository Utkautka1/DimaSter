//! Витрина скинов: поиск, выбор и что делать окну после клавиши или щелчка.

use super::Press;

/// Строк списка на экране.
pub(crate) const ROWS: usize = 14;

#[derive(Debug, Default)]
pub(crate) struct Skins {
    // первая строка — «как в настройках», дальше имена скинов
    pub(crate) names: Vec<String>,
    pub(crate) query: String,
    pub(crate) selected: usize,
    // первая видимая строка: щелчок не должен сдвигать список
    pub(crate) top: usize,
    /// Что вернётся по Esc: номер сохранённого скина.
    pub(crate) saved: usize,
    /// Названия стилей; нулевой — «как в настройках».
    pub(crate) styles: Vec<String>,
    pub(crate) style: usize,
    pub(crate) saved_style: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Показать выбранный, ничего не сохраняя.
    Preview,
    /// Сохранить выбранный; `true` — и закрыть витрину.
    Keep(bool),
    Cancel,
}

impl Skins {
    /// Номера подходящих под запрос.
    pub(crate) fn matches(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();
        let fits = |name: &String| name.to_lowercase().contains(&query);
        let all = self.names.iter().enumerate();
        all.filter(|(_, name)| fits(name)).map(|(i, _)| i).collect()
    }

    // `step` по списку найденного, по кругу
    fn step(&mut self, step: isize) {
        let found = self.matches();
        let at = found.iter().position(|&i| i == self.selected).unwrap_or(0);
        let next = (at as isize + step).rem_euclid(found.len().max(1) as isize);
        self.selected = found.get(next as usize).copied().unwrap_or(self.selected);
        self.show_selected();
    }

    /// Прокручивает список так, чтобы выбранная строка была видна.
    pub(crate) fn show_selected(&mut self) {
        let found = self.matches();
        let at = found.iter().position(|&i| i == self.selected).unwrap_or(0);
        self.top = self.top.clamp(at.saturating_sub(ROWS - 1), at);
    }

    fn search(&mut self) {
        self.selected = self.matches().first().copied().unwrap_or(self.saved);
        self.top = 0;
        self.show_selected();
    }

    /// Щелчок по видимой строке с номером `row`.
    pub(crate) fn click(&mut self, row: usize) -> Option<Outcome> {
        let index = *self.matches().get(self.top + row).filter(|_| row < ROWS)?;
        self.selected = index;
        Some(Outcome::Keep(false))
    }

    // стиль листается по кругу и показывается сразу
    pub(crate) fn turn(&mut self, step: isize) {
        let count = self.styles.len().max(1) as isize;
        self.style = (self.style as isize + step).rem_euclid(count) as usize;
    }

    pub(crate) fn press(&mut self, press: Press) -> Outcome {
        match press {
            Press::Escape | Press::Ctrl('c') => return Outcome::Cancel,
            Press::Enter { .. } => return Outcome::Keep(true),
            Press::Up | Press::Ctrl('k') => self.step(-1),
            Press::Down | Press::Ctrl('j') => self.step(1),
            Press::Left => self.turn(-1),
            Press::Right => self.turn(1),
            Press::Backspace => {
                self.query.pop();
                self.search();
            }
            Press::Char(ch) => {
                self.query.push(ch);
                self.search();
            }
            _ => {}
        }
        Outcome::Preview
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> Skins {
        Skins {
            names: (0..40).map(|i| format!("skin {i}")).collect(),
            ..Skins::default()
        }
    }

    #[test]
    fn selection_wraps_and_stays_visible() {
        let mut skins = list();
        assert_eq!(skins.press(Press::Up), Outcome::Preview);
        assert_eq!((skins.selected, skins.top), (39, 40 - ROWS));
        skins.press(Press::Down);
        assert_eq!((skins.selected, skins.top), (0, 0));
    }

    #[test]
    fn styles_turn_in_a_circle() {
        let mut skins = list();
        skins.styles = vec!["a".into(), "b".into(), "c".into()];
        assert_eq!(skins.press(Press::Left), Outcome::Preview);
        assert_eq!(skins.style, 2);
        skins.press(Press::Right);
        assert_eq!(skins.style, 0);
    }

    #[test]
    fn search_and_click_pick_rows() {
        let mut skins = list();
        skins.press(Press::Char('3'));
        assert_eq!(skins.selected, 3);
        assert_eq!(skins.click(1), Some(Outcome::Keep(false)));
        assert_eq!((skins.selected, skins.top), (13, 0));
        assert_eq!(skins.click(30), None);
    }
}
