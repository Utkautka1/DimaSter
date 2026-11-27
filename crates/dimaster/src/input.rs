//! Клавиатура: сочетания терминала и передача нажатий программе.

use dimaster_config::{Action, Binding, Chord, Mods};
use dimaster_i18n::t;
use std::time::Instant;

use winit::event::KeyEvent;
use winit::keyboard::{Key, KeyCode, KeyLocation, ModifiersState, NamedKey, PhysicalKey};
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;

use crate::clipboard::Target;
use crate::keymap::{Found, variants};
use crate::keys;
use crate::restore::open_window;
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

fn mods(state: ModifiersState) -> Mods {
    Mods {
        ctrl: state.control_key(),
        shift: state.shift_key(),
        alt: state.alt_key(),
        logo: state.super_key(),
    }
}

// сами модификаторы не прерывают цепочку
fn is_modifier(key: &Key) -> bool {
    use NamedKey::{Alt, AltGraph, Control, Shift, Super};
    matches!(key, Key::Named(Shift | Control | Alt | AltGraph | Super))
}

fn physical_name(key: PhysicalKey) -> String {
    match key {
        PhysicalKey::Code(code) => format!("{code:?}").to_lowercase(),
        PhysicalKey::Unidentified(_) => String::new(),
    }
}

fn accept_tail(tail: &str, mods: ModifiersState) -> Option<Vec<u8>> {
    if mods.is_empty() {
        return Some(tail.as_bytes().to_vec());
    }
    if mods.control_key() && !mods.alt_key() && !mods.super_key() {
        let len = tail
            .find(|c: char| !c.is_whitespace())
            .and_then(|s| tail[s..].find(char::is_whitespace).map(|e| s + e))
            .unwrap_or(tail.len());
        return Some(tail.as_bytes()[..len].to_vec());
    }
    None
}

impl View {
    pub(crate) fn key(&mut self, event: &KeyEvent) {
        let key = self.layout_independent(event);
        let text = event.text.as_deref();
        if !event.state.is_pressed() {
            if matches!(self.mode, crate::modes::Mode::Normal)
                && let Some(bytes) = self.program_bytes(event, &key, text)
            {
                self.type_bytes(bytes);
            }
            return;
        }
        if !matches!(event.logical_key, Key::Named(NamedKey::Alt)) {
            self.assist.other_key();
        }
        if self.mode_key(&key, text) {
            return;
        }
        self.handle_pressed_key(event, &key, text);
    }

    fn handle_pressed_key(&mut self, event: &KeyEvent, key: &Key, text: Option<&str>) {
        let bytes = self
            .assist_bytes(key, text)
            .or_else(|| self.program_bytes(event, key, text));
        let physical = physical_name(event.physical_key);
        if let Some(name) = key_name(key).filter(|_| !is_modifier(key)) {
            let pressed = variants(&physical, &name, mods(self.modifiers));
            // плагины спрашиваются раньше обычных привязок и могут пропустить
            if self.plugin_key(&pressed)
                || self.bound_key(pressed, bytes.clone().unwrap_or_default())
            {
                return self.render_status();
            }
        }
        match bytes {
            Some(bytes) => {
                // Enter начинает новую строку, остальное — набранный текст
                self.assist.typed = !matches!(key, Key::Named(NamedKey::Enter));
                self.type_bytes(bytes);
            }
            None => self.maybe_double_alt(key),
        }
    }

    fn assist_bytes(&mut self, key: &Key, text: Option<&str>) -> Option<Vec<u8>> {
        if let Some(bytes) = self.accept_ghost_bytes(key) {
            return Some(bytes);
        }
        if self.config.smart.autopair {
            return crate::autopair::handle(&self.term, key, text, self.modifiers);
        }
        None
    }

    fn accept_ghost_bytes(&mut self, key: &Key) -> Option<Vec<u8>> {
        let forward = matches!(key, Key::Named(NamedKey::ArrowRight | NamedKey::End));
        if !forward || !self.config.smart.autosuggest || self.term.is_alternate() {
            return None;
        }
        let prefix = crate::syntax::input_text_at_end(&self.term)?;
        let live: Vec<String> = self.term.commands().map(|c| c.command.clone()).collect();
        if let Some(tail) = self.history_store.best_suggestion(&prefix, &live) {
            return accept_tail(&tail, self.modifiers);
        }
        if self.modifiers.control_key()
            && let Some((typo, fix)) =
                crate::syntax::suggest_typo(&prefix, self.term.cwd(), &mut self.syntax)
        {
            let mut bytes = vec![0x7f; typo.chars().count()];
            bytes.extend_from_slice(fix.as_bytes());
            return Some(bytes);
        }
        None
    }

    // что должна получить программа от нажатия клавиши
    fn program_bytes(&self, event: &KeyEvent, key: &Key, text: Option<&str>) -> Option<Vec<u8>> {
        self.encoded(event, key, text)
    }

    // что получила бы программа без всяких привязок
    fn encoded(&self, event: &KeyEvent, key: &Key, text: Option<&str>) -> Option<Vec<u8>> {
        let action = match (event.state.is_pressed(), event.repeat) {
            (false, _) => keys::KeyAction::Release,
            (true, true) => keys::KeyAction::Repeat,
            (true, false) => keys::KeyAction::Press,
        };
        let modes = keys::KeyModes {
            app_cursor: self.term.modes().app_cursor,
            kitty: self.term.keyboard_flags(),
        };
        let input = keys::KeyInput {
            key,
            logical: &event.logical_key,
            physical: event.physical_key,
            text,
            mods: self.modifiers,
            action,
        };
        keys::encode(&input, modes)
    }

    /// `true` — нажатие забрали привязки.
    fn bound_key(&mut self, pressed: Vec<Chord>, bytes: Vec<u8>) -> bool {
        match self.keymap.find(&self.config.keys, &pressed) {
            Found::Prefix => {
                self.keymap.pending.push((pressed, bytes));
                true
            }
            Found::Binding(binding) => {
                let binding = binding.clone();
                self.run_binding(&binding)
            }
            Found::Nothing => {
                // оборванная цепочка уходит программе, и клавиша следом
                self.flush_sequence();
                false
            }
        }
    }

    fn run_binding(&mut self, binding: &Binding) -> bool {
        if binding.performable && !binding.actions.iter().all(|a| self.can_perform(a)) {
            self.flush_sequence();
            return false;
        }
        if binding.actions.contains(&Action::EndSequence) {
            self.flush_sequence();
        }
        self.keymap.pending.clear();
        self.keymap.used(binding);
        for action in &binding.actions {
            self.perform(action.clone());
        }
        !binding.unconsumed
    }

    fn flush_sequence(&mut self) {
        let pending = std::mem::take(&mut self.keymap.pending);
        let bytes: Vec<u8> = pending.into_iter().flat_map(|p| p.1).collect();
        if !bytes.is_empty() {
            self.type_bytes(bytes);
        }
    }

    fn can_perform(&self, action: &Action) -> bool {
        match action {
            Action::Copy => self.selecting.selection.is_some(),
            Action::ScrollPageUp | Action::ScrollTop => {
                self.scroll < self.term.screen().history_len()
            }
            Action::ScrollPageDown | Action::ScrollBottom => self.scroll > 0,
            Action::DeactivateTable | Action::DeactivateAllTables => !self.keymap.tables.is_empty(),
            _ => true,
        }
    }

    fn perform_keys(&mut self, action: Action) {
        match action {
            Action::ActivateTable { name, once } => self.keymap.activate(&name, once),
            Action::DeactivateTable => {
                self.keymap.tables.pop();
            }
            Action::DeactivateAllTables => self.keymap.tables.clear(),
            // нажатие уже забрано привязкой, цепочка отправлена до действий
            Action::Ignore | Action::EndSequence | Action::None => {}
            other => self.perform_assist(other),
        }
    }

    // сама по себе клавиша Alt ничего не отправляет
    fn maybe_double_alt(&mut self, key: &Key) {
        let alt = matches!(key, Key::Named(NamedKey::Alt));
        if self.config.smart.double_alt_sudo && alt && self.assist.alt_tap(Instant::now()) {
            self.sudo_fix();
        }
    }

    fn layout_independent(&self, event: &KeyEvent) -> Key {
        // Для numpad key_without_modifiers() сбрасывает NumLock (Mod2 в XKB)
        // и превращает цифры в End/Arrow/Insert/Delete.
        if event.location == KeyLocation::Numpad {
            return event.logical_key.clone();
        }
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

    // подсказки, история и прочие помощники
    fn perform_assist(&mut self, action: Action) {
        match action {
            Action::HintMode => self.open_hints(),
            Action::CommandHistory => self.open_history(),
            Action::PickFile => self.open_picker(crate::modes::picker::Kind::Files),
            Action::JumpDir => self.open_picker(crate::modes::picker::Kind::Dirs),
            Action::SshHosts => self.open_ssh(),
            Action::Snippets => self.open_snippets(),
            Action::Docker => self.open_docker(),
            Action::SudoFix => self.sudo_fix(),
            Action::Presentation => self.toggle_presentation(),
            Action::Minimap => self.open_minimap(),
            Action::ToggleMonitor => self.toggle_widget(false),
            Action::ToggleStatusBar => self.toggle_widget(true),
            other => log::debug!(
                "{}",
                t!(
                    "действие без обработчика: {action}",
                    action = format!("{:?}", other)
                )
            ),
        }
    }

    pub(crate) fn perform(&mut self, action: Action) {
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
            Action::Search => self.open_search(),
            Action::ViMode => self.open_vi(),
            Action::CommandPalette => self.open_palette(),
            Action::Plugin(name) => self.run_plugin(&name),
            Action::Send(text) => self.type_bytes(text.into_bytes()),
            Action::ToggleFullscreen => self.toggle_fullscreen(),
            Action::NewWindow => open_window(self.shell_directory(), None),
            Action::PromptPrevious => self.jump_prompt(false),
            Action::PromptNext => self.jump_prompt(true),
            Action::CopyLastOutput => self.copy_last_output(),
            other => self.perform_keys(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use dimaster_config::Config;

    use super::*;
    use crate::keymap::KeyState;

    #[test]
    fn default_bindings_match_winit_names() {
        let config = Config::default();
        let command = ModifiersState::CONTROL | ModifiersState::SHIFT;
        let found = |key: Key, state| {
            let pressed = variants("", &key_name(&key).unwrap(), mods(state));
            match KeyState::default().find(&config.keys, &pressed) {
                Found::Binding(binding) => Some(binding.actions.clone()),
                _ => None,
            }
        };
        let c = || Key::Character("c".into());
        assert_eq!(found(c(), command), Some(vec![Action::Copy]));
        assert_eq!(found(c(), ModifiersState::CONTROL), None);
        let page = Key::Named(NamedKey::PageUp);
        assert_eq!(
            found(page, ModifiersState::SHIFT),
            Some(vec![Action::ScrollPageUp])
        );
    }
}
