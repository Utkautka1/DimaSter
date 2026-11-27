//! Клавиши, таймеры, настройки и код завершения из плагинов.

use dimaster_config::{Effect, Plugins, Snapshot, parse_with};

const PLUGIN: &str = r##"
dimaster.bind("ctrl+shift+e", function() dimaster.send(dimaster.history()) end)
dimaster.bind("ctrl+j", function() return false end)
dimaster.every(0.01, function() dimaster.set({ window = { opacity = 0.5 }, theme = "nord" }) end)
dimaster.on("command", function(line, exit, seconds)
  dimaster.notify(line, tostring(exit) .. " " .. tostring(seconds))
end)
"##;

fn plugins(name: &str) -> Plugins {
    let directory =
        std::env::temp_dir().join(format!("dimaster-hooks-{}-{name}", std::process::id()));
    std::fs::create_dir_all(directory.join("plugins")).unwrap();
    std::fs::write(directory.join("plugins/p.lua"), PLUGIN).unwrap();
    let (plugins, errors) = Plugins::load(&directory).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    std::fs::remove_dir_all(directory).unwrap();
    plugins
}

#[test]
fn keys_can_consume_or_pass() {
    let plugins = plugins("keys");
    assert_eq!(plugins.keys().len(), 2);
    let snapshot = Snapshot {
        history: "old line".into(),
        ..Snapshot::default()
    };
    let (consumed, effects) = plugins.key(0, snapshot);
    assert!(consumed);
    assert_eq!(effects, [Effect::Send("old line".into())]);
    assert!(!plugins.key(1, Snapshot::default()).0);
}

#[test]
fn timers_set_settings_and_commands_report_exit() {
    let plugins = plugins("timers");
    assert_eq!(plugins.timers(), [1.0]);
    let [Effect::Set(literal)] = &plugins.tick(0, Snapshot::default())[..] else {
        panic!("нет настроек");
    };
    let source = "return { window = { blur = false } }";
    let config = parse_with(source, None, std::slice::from_ref(literal)).unwrap();
    assert_eq!((config.window.opacity, config.window.blur), (0.5, false));
    let effects = plugins.command_finished("make", 2, 1.5, Snapshot::default());
    let expected = Effect::Notify {
        title: "make".into(),
        body: "2 1.5".into(),
    };
    assert_eq!(effects, [expected]);
}
