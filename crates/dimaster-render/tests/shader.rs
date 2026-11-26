//! Шейдер проверяется без видеокарты: ошибка в нём роняет окно на старте.

use naga::valid::{Capabilities, ValidationFlags, Validator};

#[test]
fn shader_parses_and_validates() {
    let source = include_str!("../src/shader.wgsl");
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
    Validator::new(ValidationFlags::all(), Capabilities::default())
        .validate(&module)
        .unwrap();
}
