//! Поиск привязки: цепочки нажатий, таблицы и `catch_all`.
//!
//! Чистая логика без окна: что делать с найденным, решает `input`.

use dimaster_config::{Action, Binding, Chord};

const CATCH_ALL: &str = "catch_all";

/// Начатая цепочка и включённые таблицы.
#[derive(Debug, Default)]
pub(crate) struct KeyState {
    // нажатия цепочки и байты, которые получила бы программа без привязки
    pub(crate) pending: Vec<(Vec<Chord>, Vec<u8>)>,
    // внутренняя таблица — последняя; `true` — одноразовая
    pub(crate) tables: Vec<(String, bool)>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Found<'a> {
    Binding(&'a Binding),
    /// Начало более длинной цепочки.
    Prefix,
    Nothing,
}

// `want` из привязки подходит под один из вариантов нажатия
fn matches(want: &Chord, pressed: &[Chord]) -> bool {
    pressed.contains(want)
}

fn starts(binding: &Binding, typed: &[&[Chord]]) -> bool {
    binding.chords.len() >= typed.len()
        && binding
            .chords
            .iter()
            .zip(typed)
            .all(|(want, got)| matches(want, got))
}

// привязка «none» только снимает сочетание
fn live(binding: &Binding) -> bool {
    binding.actions != [Action::None]
}

fn find_in<'a>(bindings: &'a [Binding], table: &str, typed: &[&[Chord]]) -> Found<'a> {
    let here = || bindings.iter().filter(|b| b.table == table && live(b));
    // варианты нажатия идут по убыванию точности
    let exact = typed
        .last()
        .into_iter()
        .flat_map(|last| last.iter())
        .find_map(|chord| {
            here().find(|b| {
                b.chords.len() == typed.len() && starts(b, typed) && b.chords.last() == Some(chord)
            })
        });
    match exact {
        Some(binding) => Found::Binding(binding),
        None if here().any(|b| b.chords.len() > typed.len() && starts(b, typed)) => Found::Prefix,
        None => Found::Nothing,
    }
}

/// Варианты одного нажатия: физическая клавиша, символ, затем `catch_all`.
pub(crate) fn variants(physical: &str, logical: &str, mods: dimaster_config::Mods) -> Vec<Chord> {
    let mut out = vec![Chord::new(physical, mods), Chord::new(logical, mods)];
    out.push(Chord::new(CATCH_ALL, mods));
    if mods != Default::default() {
        out.push(Chord::new(CATCH_ALL, Default::default()));
    }
    out
}

impl KeyState {
    /// Ищет от внутренней таблицы к основной.
    pub(crate) fn find<'a>(&self, bindings: &'a [Binding], pressed: &[Chord]) -> Found<'a> {
        let mut typed: Vec<&[Chord]> = self.pending.iter().map(|p| p.0.as_slice()).collect();
        typed.push(pressed);
        let tables = self.tables.iter().rev().map(|t| t.0.as_str()).chain([""]);
        for table in tables {
            let found = find_in(bindings, table, &typed);
            if found != Found::Nothing {
                return found;
            }
        }
        Found::Nothing
    }

    /// Повторное включение внутренней таблицы пропускается.
    pub(crate) fn activate(&mut self, name: &str, once: bool) {
        if self.tables.last().is_none_or(|t| t.0 != name) {
            self.tables.push((name.to_owned(), once));
        }
    }

    // одноразовая таблица выключается первой сработавшей привязкой не из catch_all
    pub(crate) fn used(&mut self, binding: &Binding) {
        let catch_all = binding.chords.last().is_some_and(|c| c.key == CATCH_ALL);
        let once = self
            .tables
            .iter()
            .rposition(|t| t.1 && t.0 == binding.table);
        if let Some(index) = once.filter(|_| !catch_all) {
            self.tables.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use dimaster_config::{Config, Mods, parse};

    use super::*;

    fn config() -> Config {
        let source = r#"return { clear_default_keys = true, keys = {
            { key = "ctrl+a>n", action = "new_window" },
            { key = "catch_all", table = "t", action = "ignore" },
            { key = "x", table = "t", action = "copy" },
        } }"#;
        parse(source, None).unwrap()
    }

    fn press(key: &str, ctrl: bool) -> Vec<Chord> {
        variants(
            "",
            key,
            Mods {
                ctrl,
                ..Mods::default()
            },
        )
    }

    #[test]
    fn sequences_wait_for_their_end() {
        let (config, mut state) = (config(), KeyState::default());
        assert_eq!(state.find(&config.keys, &press("a", true)), Found::Prefix);
        state.pending.push((press("a", true), vec![1]));
        let found = state.find(&config.keys, &press("n", false));
        assert!(matches!(found, Found::Binding(b) if b.actions == [Action::NewWindow]));
        assert_eq!(state.find(&config.keys, &press("q", false)), Found::Nothing);
    }

    #[test]
    fn tables_come_first_and_catch_the_rest() {
        let (config, mut state) = (config(), KeyState::default());
        state.activate("t", true);
        state.activate("t", false);
        assert_eq!(state.tables.len(), 1);
        let Found::Binding(copy) = state.find(&config.keys, &press("x", false)) else {
            panic!("нет привязки");
        };
        assert_eq!(copy.actions, [Action::Copy]);
        let Found::Binding(any) = state.find(&config.keys, &press("z", true)) else {
            panic!("нет catch_all");
        };
        assert_eq!(any.actions, [Action::Ignore]);
        state.used(any);
        assert_eq!(state.tables.len(), 1);
        state.used(copy);
        assert!(state.tables.is_empty());
    }
}
