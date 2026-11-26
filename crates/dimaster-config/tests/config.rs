//! Разбор конфига, тем, обратная запись и импорт.

use dimaster_config::import::{Source, detect};
use dimaster_config::{
    Action, Config, CursorStyle, Decorations, Mods, builtin_names, load, parse, scaffold,
};

#[test]
fn empty_table_gives_defaults() {
    assert_eq!(parse("return {}", None).unwrap(), Config::default());
}

#[test]
fn reads_settings_and_overrides_theme_colors() {
    let source = r##"
        local big = true
        return {
          theme = "nord", colors = { background = "#000" },
          font = { family = "Mono", size = big and 18 or 10 },
          padding = 3, cursor = "bar", scrollback = 5, rtl = false,
          keys = { { key = "C", mods = "ctrl+shift", action = "none" } },
        }"##;
    let config = parse(source, None).unwrap();
    assert_eq!(
        (config.theme.background, config.theme.foreground),
        (0, 0xd8dee9)
    );
    assert_eq!((config.font_size, config.padding), (18.0, (3, 3)));
    assert_eq!(
        (config.cursor, config.scrollback, config.rtl),
        (CursorStyle::Bar, 5, false)
    );
    let mods = Mods::parse("ctrl+shift").unwrap();
    let copy: Vec<_> = config
        .keys
        .iter()
        .filter(|b| b.key == "c" && b.mods == mods)
        .collect();
    assert_eq!((copy.len(), copy[0].action.clone()), (1, Action::None));
}

#[test]
fn reports_mistakes() {
    for source in [
        "return 1",
        "return { cursor = 'round' }",
        "return { theme = 'нет такой' }",
        "return { colors = { background = 'red' } }",
        "return { font = { size = 0 } }",
        "return {",
    ] {
        assert!(parse(source, None).is_err(), "принят {source}");
    }
}

#[test]
fn builtin_themes_are_valid() {
    for name in builtin_names() {
        parse(&format!("return {{ theme = '{name}' }}"), None).expect(name);
    }
}

fn temporary(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("dimaster-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn write_tree(directory: &std::path::Path, config: &Config) {
    for file in scaffold(config) {
        let path = directory.join(file.path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, file.content).unwrap();
    }
}

fn custom() -> Config {
    let mut config = Config {
        font_family: Some("A \"B\" \\C".into()),
        ..Config::default()
    };
    config.theme.ansi[5] = 0x123456;
    config.theme.selection_text = Some(0x010101);
    config.shell = vec!["/bin/zsh".into(), "-l".into()];
    config.padding = (25, 20);
    config.window.decorations = Decorations::None;
    config.window.opacity = 0.5;
    config.effects.cursor_trail = true;
    config.restore_session = true;
    config.session_history = 10;
    config.remote_control = false;
    config
}

#[test]
fn generated_tree_reads_back() {
    let directory = temporary("tree");
    write_tree(&directory, &custom());
    // переменные окружения теста не должны подключать лишние профили
    let loaded = load(&directory.join("init.lua")).unwrap();
    assert_eq!(loaded, custom());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn later_layers_win_and_keys_accumulate() {
    let directory = temporary("layers");
    write_tree(&directory, &Config::default());
    let write = |name: &str, text: &str| std::fs::write(directory.join(name), text).unwrap();
    write(
        "settings/keys.lua",
        "return { keys = { { key = 'a', mods = 'ctrl', action = 'copy' } } }",
    );
    write(
        "local.lua",
        "return { font = { size = 20 }, keys = { { key = 'b', mods = 'ctrl', action = 'paste' } } }",
    );
    let config = load(&directory.join("init.lua")).unwrap();
    assert_eq!((config.font_size, config.font_dirs.len()), (20.0, 0));
    let has = |key: &str| config.keys.iter().any(|b| b.key == key);
    assert!(has("a") && has("b"));
    write("local.lua", "return 5");
    assert!(load(&directory.join("init.lua")).is_err());
    std::fs::remove_dir_all(directory).unwrap();
}

// временный «~/.config» с настройками двух терминалов
fn sample_config_home() -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!("dimaster-import-{}", std::process::id()));
    let files = [
        (
            "ghostty/themes/mine",
            "background = #010203\nforeground = #040506\n",
        ),
        (
            "ghostty/config",
            "foreground = #ffffff\ntheme = mine\nfont-family = \"F\"\npalette = 1=#ff0000\n",
        ),
        (
            "kitty/kitty.conf",
            "include extra.conf\nfont_family F Mono Bold\ncolor2 #00ff00\n",
        ),
        ("kitty/extra.conf", "font_size 16\nbackground #111111\n"),
    ];
    for (path, text) in files {
        let path = home.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    home
}

#[test]
fn imports_ghostty_and_kitty_with_includes() {
    let home = sample_config_home();
    assert_eq!(detect(&home), [Source::Ghostty, Source::Kitty]);
    let mut ghostty = Config::default();
    Source::Ghostty.apply(&home, &mut ghostty).unwrap();
    assert_eq!(
        (ghostty.theme.background, ghostty.theme.foreground),
        (0x010203, 0xffffff)
    );
    assert_eq!(
        (ghostty.font_family.as_deref(), ghostty.theme.ansi[1]),
        (Some("F"), 0xff0000)
    );
    let mut kitty = Config::default();
    Source::Kitty.apply(&home, &mut kitty).unwrap();
    assert_eq!(
        (kitty.font_family.as_deref(), kitty.font_size),
        (Some("F Mono"), 16.0)
    );
    assert_eq!(
        (kitty.theme.background, kitty.theme.ansi[2]),
        (0x111111, 0x00ff00)
    );
    std::fs::remove_dir_all(home).unwrap();
}
