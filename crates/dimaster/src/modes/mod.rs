//! Режимы, в которых клавиатура принадлежит терминалу, а не программе.

pub(crate) mod hint;
pub(crate) mod history;
pub(crate) mod launch;
pub(crate) mod minimap;
pub(crate) mod palette;
pub(crate) mod paste;
pub(crate) mod picker;
pub(crate) mod search;
pub(crate) mod skins;
pub(crate) mod vi;

use winit::keyboard::{Key, ModifiersState, NamedKey};

/// Клавиша в виде, удобном для режимов.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Press {
    Char(char),
    Ctrl(char),
    Enter { shift: bool },
    Escape,
    Backspace,
    Up,
    Down,
    Left,
    Right,
}

fn named(key: NamedKey, shift: bool) -> Option<Press> {
    Some(match key {
        NamedKey::Enter => Press::Enter { shift },
        NamedKey::Escape => Press::Escape,
        NamedKey::Backspace => Press::Backspace,
        NamedKey::ArrowUp => Press::Up,
        NamedKey::ArrowDown => Press::Down,
        NamedKey::ArrowLeft => Press::Left,
        NamedKey::ArrowRight => Press::Right,
        _ => return None,
    })
}

/// `text` — то, что напечатала бы клавиша с учётом раскладки.
pub(crate) fn press(key: &Key, text: Option<&str>, mods: ModifiersState) -> Option<Press> {
    if let Key::Named(code) = key
        && let Some(press) = named(*code, mods.shift_key())
    {
        return Some(press);
    }
    if mods.control_key() {
        let Key::Character(chars) = key else {
            return None;
        };
        return chars.chars().next().map(Press::Ctrl);
    }
    text?
        .chars()
        .next()
        .filter(|c| !c.is_control())
        .map(Press::Char)
}

#[derive(Debug, Default)]
pub(crate) enum Mode {
    #[default]
    Normal,
    Search(search::Search),
    Vi(vi::Vi),
    Palette(palette::Palette),
    Hint(hint::Hints),
    History(history::History),
    Picker(picker::Picker),
    Skins(skins::Skins),
    Minimap(minimap::Minimap),
    Paste(paste::PasteConfirm),
    CloseConfirm,
}
