//! Пределы действуют уже при разборе того конфига, где заданы.
//!
//! Пределы общие на процесс, поэтому проверка живёт в отдельном тестовом файле.

use dimaster_config::parse;

#[test]
fn raised_bound_applies_to_the_same_config() {
    let source = "return { limits = { opacity_max_percent = 200 }, window = { opacity = 1.5 } }";
    let config = parse(source, None).expect("конфиг");
    assert_eq!(config.window.opacity, 1.5);
}
