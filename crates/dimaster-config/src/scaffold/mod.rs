//! Дерево файлов настроек, которое мастер создаёт в `~/.config/dimaster`.
//!
//! Статические файлы (загрузчик, слои, объединение) лежат рядом как обычный
//! Lua; файлы со значениями генерируются из [`Config`].

mod render;

use dimaster_i18n::tr;

use crate::Config;

/// Один файл дерева.
#[derive(Debug)]
pub struct File {
    /// Путь относительно каталога настроек.
    pub path: &'static str,
    pub content: String,
    /// Пользовательский файл: создаётся один раз и больше не перезаписывается.
    pub owned_by_user: bool,
}

const FIXED: [(&str, &str, bool); 12] = [
    ("draw.lua", include_str!("files/draw.lua"), true),
    ("ui.lua", include_str!("files/ui.lua"), true),
    ("statusbar.lua", include_str!("files/statusbar.lua"), true),
    ("fetch.lua", include_str!("files/fetch.lua"), true),
    (
        "plugins/example.lua",
        include_str!("files/plugin.lua"),
        true,
    ),
    ("init.lua", include_str!("files/init.lua"), false),
    ("lib/layers.lua", include_str!("files/layers.lua"), false),
    ("lib/merge.lua", include_str!("files/merge.lua"), false),
    ("local.lua", include_str!("files/local.lua"), true),
    (
        "profiles/presentation.lua",
        include_str!("files/presentation.lua"),
        true,
    ),
    ("profiles/ssh.lua", include_str!("files/ssh.lua"), true),
    (
        "profiles/desktop/hyprland.lua",
        include_str!("files/desktop.lua"),
        true,
    ),
];

fn managed(path: &'static str, content: String) -> File {
    File {
        path,
        content,
        owned_by_user: false,
    }
}

// файлы со значениями из конфига
fn generated(config: &Config) -> Vec<File> {
    let mut files = vec![
        managed("settings/appearance.lua", render::appearance(config)),
        managed("settings/font.lua", render::font(config)),
        managed("settings/window.lua", render::window(config)),
        managed("settings/effects.lua", render::effects(config)),
        managed("settings/behavior.lua", render::behavior(config)),
        managed(
            "settings/keys.lua",
            include_str!("files/keys.lua").to_owned(),
        ),
    ];
    if config.theme != crate::Theme::default() {
        files.push(managed("themes/imported.lua", render::theme(&config.theme)));
    }
    files
}

/// Полное дерево настроек, равнозначное `config`.
pub fn scaffold(config: &Config) -> Vec<File> {
    let mut files: Vec<File> = FIXED
        .iter()
        .map(|&(path, content, owned_by_user)| File {
            path,
            content: content.to_owned(),
            owned_by_user,
        })
        .collect();
    files.extend(generated(config));
    for file in &mut files {
        file.content = translated(&file.content);
    }
    files
}

// строки с русскими комментариями заменяются переводом целиком
fn translated(content: &str) -> String {
    let lines: Vec<String> = content.lines().map(tr).collect();
    lines.join("\n") + "\n"
}
