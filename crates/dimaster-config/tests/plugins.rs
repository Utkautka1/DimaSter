//! Загрузка плагинов, события, команды и защита от зависаний.

use dimaster_config::{Effect, Plugins, Snapshot};

fn load(files: &[(&str, &str)]) -> (Plugins, Vec<String>, std::path::PathBuf) {
    let unique = format!("dimaster-plugins-{}-{}", std::process::id(), files[0].0);
    let directory = std::env::temp_dir().join(unique);
    std::fs::create_dir_all(directory.join("plugins")).unwrap();
    for (name, source) in files {
        std::fs::write(directory.join("plugins").join(name), source).unwrap();
    }
    let (plugins, errors) = Plugins::load(&directory).unwrap();
    (plugins, errors, directory)
}

#[test]
fn commands_events_and_snapshot() {
    let (plugins, errors, directory) = load(&[(
        "a.lua",
        "dimaster.command('hi', 'Привет', function() dimaster.send(dimaster.cwd()) end)
         dimaster.on('title', function(t) dimaster.notify('t', t) end)",
    )]);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(plugins.commands(), [("hi".to_owned(), "Привет".to_owned())]);
    let snapshot = Snapshot {
        cwd: "/tmp".into(),
        ..Snapshot::default()
    };
    assert_eq!(plugins.run("hi", snapshot), [Effect::Send("/tmp".into())]);
    let notify = Effect::Notify {
        title: "t".into(),
        body: "x".into(),
    };
    assert_eq!(plugins.emit("title", "x", Snapshot::default()), [notify]);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn broken_and_endless_plugins_do_not_block_others() {
    let (plugins, errors, directory) = load(&[
        ("b.lua", "this is not lua"),
        ("c.lua", "while true do end"),
        (
            "d.lua",
            "dimaster.on('startup', function() error('x') end)
             dimaster.on('startup', function() while true do end end)
             dimaster.command('ok', 'ok', function() dimaster.action('copy') end)",
        ),
    ]);
    assert_eq!(errors.len(), 2);
    assert!(plugins.emit("startup", "", Snapshot::default()).is_empty());
    assert_eq!(
        plugins.run("ok", Snapshot::default()),
        [Effect::Action("copy".into())]
    );
    std::fs::remove_dir_all(directory).unwrap();
}
