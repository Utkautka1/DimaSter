//! Все скины загружаются и накладываются поверх настроек.

use dimaster_config::{parse_with, skin_literal, skins};

#[test]
fn fifty_distinct_skins_override_the_config() {
    let all = skins();
    let mut names: Vec<&str> = all.iter().map(|s| s.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!((all.len(), names.len()), (817, 817));
    let source = r##"return { theme = "nord", colors = { background = "#010203" } }"##;
    for skin in all {
        let literal = skin_literal(skin.name).unwrap();
        let config = parse_with(source, None, &[literal]).unwrap();
        assert_eq!(config.theme, skin.theme, "{}", skin.name);
    }
    assert!(skin_literal("нет такого").is_none());
}
