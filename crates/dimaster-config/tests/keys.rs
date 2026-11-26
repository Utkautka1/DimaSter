//! Разбор привязок: цепочки, приставки, таблицы и вытеснение.

use dimaster_config::{Action, Binding, Config, parse};

const SOURCE: &str = r#"return { clear_default_keys = true, keys = {
    { key = "ctrl+a>n", action = "new_window" },
    { key = "ctrl+a>t", action = { "scroll_top", "text:\\x15", "csi:A" } },
    { key = "unconsumed:performable:ctrl+shift+key_c", action = "copy" },
    { key = "j", table = "move", action = "activate_key_table_once:other" },
    { key = "+", mods = "ctrl", action = "font_increase" },
} }"#;

fn names(binding: &Binding) -> Vec<&str> {
    binding.chords.iter().map(|c| c.key.as_str()).collect()
}

#[test]
fn sequences_chains_prefixes_and_tables() {
    let keys = parse(SOURCE, None).unwrap().keys;
    assert_eq!(keys.len(), 5);
    assert_eq!(names(&keys[0]), ["a", "n"]);
    let chain = [
        Action::ScrollTop,
        Action::Send("\x15".into()),
        Action::Send("\x1b[A".into()),
    ];
    assert_eq!(keys[1].actions, chain);
    assert!(keys[2].unconsumed && keys[2].performable);
    assert_eq!(
        (names(&keys[2]), keys[3].table.as_str()),
        (vec!["keyc"], "move")
    );
    assert_eq!(names(&keys[4]), ["+"]);
}

#[test]
fn rebinding_a_prefix_removes_its_sequences() {
    let mut config = Config::default();
    let source = parse(SOURCE, None).unwrap();
    config.keys.clear();
    config.bind(source.keys[0].clone());
    config.bind(source.keys[1].clone());
    let mut prefix = source.keys[0].clone();
    prefix.chords.truncate(1);
    config.bind(prefix.clone());
    assert_eq!(config.keys, [prefix]);
    assert!(
        parse(
            r#"return { keys = { { key = "global:ctrl+a", action = "copy" } } }"#,
            None
        )
        .is_err()
    );
}
