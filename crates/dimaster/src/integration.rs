//! Автоматическая интеграция с shell: метки команд без правки dotfiles.
//!
//! Скрипты встроены в бинарник и раскладываются в каталог данных. Интеграция
//! подключается, только когда запускается сам shell без аргументов: чужие
//! команды (`-e`, свои флаги) не изменяются.

use std::io;
use std::path::{Path, PathBuf};

const FILES: [(&str, &str); 3] = [
    ("zsh/.zshenv", include_str!("../../../assets/shell/zshenv")),
    ("bashrc", include_str!("../../../assets/shell/bashrc")),
    (
        "dimaster.fish",
        include_str!("../../../assets/shell/dimaster.fish"),
    ),
];

/// Что добавить к запуску shell.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Injection {
    pub(crate) args: Vec<String>,
    pub(crate) env: Vec<(String, String)>,
}

fn directory() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(std::env::var_os("HOME")?).join(".local/share")))?;
    Some(base.join("dimaster/shell"))
}

fn install(directory: &Path) -> io::Result<()> {
    for (name, content) in FILES {
        let path = directory.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
    }
    Ok(())
}

fn plan(program: &str, directory: &Path, zdotdir: Option<String>) -> Injection {
    let file = |name: &str| directory.join(name).to_string_lossy().into_owned();
    let mut injection = Injection::default();
    match Path::new(program).file_name().and_then(|n| n.to_str()) {
        Some("zsh") => {
            injection.env.push(("ZDOTDIR".into(), file("zsh")));
            injection
                .env
                .extend(zdotdir.map(|v| ("DIMASTER_ZDOTDIR".into(), v)));
        }
        Some("bash") => injection.args = vec!["--rcfile".into(), file("bashrc")],
        Some("fish") => {
            let source = format!("source '{}'", file("dimaster.fish").replace('\'', "\\'"));
            injection.args = vec!["--init-command".into(), source];
        }
        _ => {}
    }
    injection
}

/// Пустой результат — запускать как есть.
pub(crate) fn inject(args: &[String]) -> Injection {
    let [program] = args else {
        return Injection::default();
    };
    let Some(directory) = directory() else {
        return Injection::default();
    };
    if let Err(e) = install(&directory) {
        log::warn!("Интеграция с shell отключена: {e}");
        return Injection::default();
    }
    plan(program, &directory, std::env::var("ZDOTDIR").ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_shell_gets_its_own_hook() {
        let dir = Path::new("/d");
        let zsh = plan("/usr/bin/zsh", dir, Some("/z".into()));
        assert_eq!(zsh.env[0], ("ZDOTDIR".into(), "/d/zsh".into()));
        assert_eq!(zsh.env[1], ("DIMASTER_ZDOTDIR".into(), "/z".into()));
        assert_eq!(plan("bash", dir, None).args, ["--rcfile", "/d/bashrc"]);
        assert_eq!(plan("fish", dir, None).args[0], "--init-command");
        assert_eq!(plan("nvim", dir, None), Injection::default());
    }
}
