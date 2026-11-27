//! Ключ `snippets`.

use dimaster_config::parse;

#[test]
fn names_default_to_the_command_and_newlines_are_refused() {
    let source =
        r#"return { snippets = { { command = "ls {{dir}}" }, { name = "x", command = "y" } } }"#;
    let snippets = parse(source, None).unwrap().snippets;
    assert_eq!(
        (snippets[0].name.as_str(), snippets[1].name.as_str()),
        ("ls {{dir}}", "x")
    );
    assert!(parse(r#"return { snippets = { { command = "a\nrm b" } } }"#, None).is_err());
}
