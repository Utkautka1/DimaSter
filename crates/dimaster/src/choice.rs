//! Пункт стиля витрины: встроенный стиль или свой фон поверх него.

use dimaster_config::{Style, style, styles, theme_literal};
use dimaster_i18n::{t, tr};

use crate::photos::{Photo, photos};

// так записан свой фон в файле выбора
const PHOTO: &str = "photo:";

#[derive(Clone, Copy)]
pub(crate) struct Choice {
    /// Окно, курсор, строка статуса и приглашение.
    pub(crate) style: &'static Style,
    pub(crate) photo: Option<&'static Photo>,
}

impl Choice {
    fn builtin(style: &'static Style) -> Self {
        Self { style, photo: None }
    }

    fn own(photo: &'static Photo) -> Self {
        Self {
            style: photo.base,
            photo: Some(photo),
        }
    }

    pub(crate) fn id(&self) -> String {
        match self.photo {
            Some(photo) => format!("{PHOTO}{}", photo.name),
            None => self.style.id.to_owned(),
        }
    }

    pub(crate) fn title(&self) -> String {
        match self.photo {
            Some(photo) => t!("Фон: {name}", name = photo.name),
            None => tr(self.style.title),
        }
    }

    /// Слои настроек; цвета фона уступают выбранному скину.
    pub(crate) fn layers(&self, skin: Option<String>) -> Vec<String> {
        let own = || Some(theme_literal(&self.photo?.loaded()?.theme));
        let colors = skin.or_else(own);
        let shape = Some(self.style.literal.to_owned());
        colors.into_iter().chain(shape).collect()
    }
}

/// Все пункты в порядке витрины: встроенные, затем свои фоны.
pub(crate) fn choices() -> Vec<Choice> {
    let builtin = styles().iter().map(Choice::builtin);
    builtin.chain(photos().iter().map(Choice::own)).collect()
}

/// Пункт по записи из файла выбора.
pub(crate) fn choice(id: &str) -> Option<Choice> {
    match id.strip_prefix(PHOTO) {
        Some(name) => photos().iter().find(|p| p.name == name).map(Choice::own),
        None => style(id).map(Choice::builtin),
    }
}
