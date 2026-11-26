//! Клавиатура: сочетания терминала и передача нажатий программе.

use dimaster_config::{Action, Binding, Mods};
use winit::event::KeyEvent;
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

use crate::clipboard::Target;
use crate::keys;
use crate::view::View;

// порядок совпадает с алфавитом в latin_letter
const LETTER_KEYS: [KeyCode; 26] = {
    use KeyCode::*;
    [
        KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO,
        KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,
    ]
};

/// Латинская буква физической клавиши при любой раскладке.
fn latin_letter(code: KeyCode) -> Option<&'static str> {
    let index = LETTER_KEYS.iter().position(|&c| c == code)?;
    "abcdefghijklmnopqrstuvwxyz".get(index..=index)
}

// имя клавиши в том виде, в каком его пишут в конфиге
fn key_name(key: &Key) -> Option<String> {
    match key {
        Key::Character(text) => Some(text.to_lowercase()),
        Key::Named(NamedKey::Space) => Some("space".to_owned()),
        Key::Named(named) => Some(format!("{named:?}").to_lowercase()),
        _ => None,
    }
}

fn find_action(bindings: &[Binding], key: &Key, state: ModifiersState) -> Option<Action> {
    let name = key_name(key)?;
    let mods = Mods {
        ctrl: state.control_key(),
        shift: state.shift_key(),
        alt: state.alt_key(),
        logo: state.super_key(),
    };
    let binding = bindings.iter().find(|b| b.mods == mods && b.key == name)?;
    Some(binding.action).filter(|&a| a != Action::None)
}

impl View {
    pub(crate) fn key(&mut self, event: &KeyEvent) {
        // без модификаторов: Ctrl+C должен работать и на русской раскладке
        let key = self.layout_independent(event);
        if let Some(action) = find_action(&self.config.keys, &key, self.modifiers) {
            return self.perform(action);
        }
        let modes = keys::KeyModes {
            app_cursor: self.term.modes().app_cursor,
            disambiguate: self.term.keyboard_flags() & 1 != 0,
        };
        let text = event.text.as_deref();
        if let Some(bytes) = keys::encode(&key, text, self.modifiers, modes) {
            self.type_bytes(bytes);
        }
    }

    fn layout_independent(&self, event: &KeyEvent) -> Key {
        let key = event.key_without_modifiers();
        let latin = matches!(&key, Key::Character(c) if c.is_ascii());
        let chord = self.modifiers.control_key() || self.modifiers.super_key();
        if !chord || latin {
            return key;
        }
        match event.physical_key {
            PhysicalKey::Code(code) => latin_letter(code).map_or(key, |c| Key::Character(c.into())),
            PhysicalKey::Unidentified(_) => key,
        }
    }

    fn perform(&mut self, action: Action) {
        let page = self.term.screen().rows().len() as isize;
        match action {
            Action::Copy => self.copy(Target::Clipboard),
            Action::Paste => self.paste(Target::Clipboard),
            Action::PastePrimary => self.paste(Target::Primary),
            Action::ScrollPageUp => self.scroll_by(page),
            Action::ScrollPageDown => self.scroll_by(-page),
            Action::ScrollTop => self.scroll_by(isize::MAX),
            Action::ScrollBottom => self.scroll_by(isize::MIN),
            Action::FontIncrease => self.zoom_font(Some(1.0)),
            Action::FontDecrease => self.zoom_font(Some(-1.0)),
            Action::FontReset => self.zoom_font(None),
            Action::ReloadConfig => self.reload(),
            Action::None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use dimaster_config::Config;

    use super::*;

    #[test]
    fn default_bindings_match_winit_names() {
        let keys = Config::default().keys;
        let command = ModifiersState::CONTROL | ModifiersState::SHIFT;
        let c = Key::Character("c".into());
        assert_eq!(find_action(&keys, &c, command), Some(Action::Copy));
        assert_eq!(find_action(&keys, &c, ModifiersState::CONTROL), None);
        let page = Key::Named(NamedKey::PageUp);
        assert_eq!(
            find_action(&keys, &page, ModifiersState::SHIFT),
            Some(Action::ScrollPageUp)
        );
    }
}
