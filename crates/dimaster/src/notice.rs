//! Короткая плашка в углу окна: «скопировано».

use std::time::{Duration, Instant};

use dimaster_config::{Align, Anchor};
use dimaster_i18n::tr;
use unicode_width::UnicodeWidthStr;

use crate::clipboard::Target;
use crate::overlay::{Drawn, fill};
use crate::view::View;

/// Что показывать и до какого момента.
#[derive(Debug, Default)]
pub(crate) struct Notice {
    shown: Option<(String, Instant)>,
}

impl Notice {
    /// Когда плашка должна исчезнуть; `None` — её уже нет.
    pub(crate) fn expires(&self, now: Instant) -> Option<Instant> {
        self.shown
            .as_ref()
            .map(|s| s.1)
            .filter(|&until| until > now)
    }
}

impl View {
    /// Кладёт текст в буфер по действию пользователя и сообщает об этом.
    pub(crate) fn put_clipboard(&mut self, target: Target, text: String) {
        let message = message(&tr(&self.config.ui.notice.text), &text, target);
        if !self.clipboard.set(target, text) {
            return;
        }
        let style = &self.config.ui.notice;
        if style.copy {
            let until = Instant::now() + Duration::from_millis(u64::from(style.duration));
            self.notice.shown = Some((message, until));
            self.window.request_redraw();
        }
    }

    pub(crate) fn notice_panel(&self) -> Drawn {
        let live = self.notice.expires(Instant::now());
        let Some((text, _)) = self.notice.shown.as_ref().filter(|_| live.is_some()) else {
            return Drawn::default();
        };
        let look = self.config.ui.notice.look.unwrap_or(self.config.ui.panel);
        let lines: Vec<_> = text.lines().map(|l| (l.to_owned(), false)).collect();
        let columns = lines.iter().map(|l| l.0.width()).max().unwrap_or(0);
        let size = self.styled_size(look, columns, lines.len());
        self.styled_panel(look, self.notice_corner(size), columns, &lines)
    }

    fn notice_corner(&self, (width, height): (u32, u32)) -> (u32, u32) {
        let (style, window) = (&self.config.ui.notice, self.window.inner_size());
        let (dx, dy) = style.margin;
        let x = match style.side {
            Align::Left => dx,
            Align::Center => window.width.saturating_sub(width) / 2,
            Align::Right => window.width.saturating_sub(width + dx),
        };
        let y = match style.position {
            Anchor::Top => self.content_top() + dy,
            Anchor::Center => window.height.saturating_sub(height) / 2,
            Anchor::Bottom => window
                .height
                .saturating_sub(height + self.content_bottom() + dy),
        };
        (x, y)
    }
}

// начало первой строки скопированного: плашка не должна разрастаться
fn preview(text: &str) -> String {
    let line = text.trim().lines().next().unwrap_or_default();
    let mut short: String = line.chars().filter(|c| !c.is_control()).take(30).collect();
    if short.len() < line.len() {
        short.push('…');
    }
    short
}

// подстановки; `{preview}` идёт последней, чтобы скопированные `{…}` не раскрылись
fn message(template: &str, copied: &str, target: Target) -> String {
    let count = copied.chars().count().to_string();
    let lines = copied.lines().count().to_string();
    let name = match target {
        Target::Clipboard => "clipboard",
        Target::Primary => "primary",
    };
    let values = [
        ("count", count.as_str()),
        ("lines", lines.as_str()),
        ("target", name),
        ("preview", &preview(copied)),
    ];
    fill(template, &values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_filled_from_what_was_copied() {
        let text = message(
            "{count}/{lines} {target}\n{preview}",
            "ab{count}\ncd",
            Target::Primary,
        );
        assert_eq!(text, "12/2 primary\nab{count}");
        assert!(preview(&"x".repeat(50)).ends_with('…'));
    }
}
