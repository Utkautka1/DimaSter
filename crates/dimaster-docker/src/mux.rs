//! Одно SSH-соединение на сервер вместо нового на каждый запрос.
//!
//! `docker -H ssh://…` запускает `ssh` из `PATH` и не принимает его ключи,
//! поэтому перед настоящим `ssh` в `PATH` ставится обёртка с `ControlMaster`.
//! `~/.ssh/config` не меняется; соединение закрывается само спустя минуту
//! после последнего запроса.

use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// первый настоящий `ssh` в PATH, не считая нашей обёртки
fn real_ssh(path: &OsStr, own: &Path) -> Option<PathBuf> {
    std::env::split_paths(path)
        .filter(|directory| directory != own)
        .map(|directory| directory.join("ssh"))
        .find(|file| file.is_file())
}

// путь в одинарных кавычках shell; с кавычкой внутри обёртку не строим
fn quoted(path: &Path) -> Option<String> {
    let text = path.to_str().filter(|t| !t.contains(['\'', '"']))?;
    Some(format!("'{text}'"))
}

fn script(ssh: &Path, directory: &Path) -> Option<String> {
    // %C — хэш хоста, порта и пользователя: имя сокета остаётся коротким
    let control = quoted(&directory.join("%C"))?;
    let ssh = quoted(ssh)?;
    let persist = crate::timing::get().ssh_persist;
    Some(format!(
        "#!/bin/sh\nexec {ssh} -o ControlMaster=auto -o ControlPath={control} \
         -o ControlPersist={persist} -o ServerAliveInterval=15 \"$@\"\n"
    ))
}

// запись через временный файл: другое окно не увидит половину обёртки
fn write(directory: &Path, text: &str) -> io::Result<()> {
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(directory)?;
    let temporary = directory.join(format!("ssh.{}", std::process::id()));
    let mut options = std::fs::OpenOptions::new();
    let options = options.write(true).create(true).truncate(true).mode(0o700);
    io::Write::write_all(&mut options.open(&temporary)?, text.as_bytes())?;
    std::fs::rename(temporary, directory.join("ssh"))
}

/// `PATH`, в котором `ssh` — обёртка из каталога `directory`.
fn shimmed(path: &OsStr, directory: &Path) -> Option<OsString> {
    let text = script(&real_ssh(path, directory)?, directory)?;
    write(directory, &text).ok()?;
    let rest = std::env::split_paths(path).filter(|p| p != directory);
    std::env::join_paths(std::iter::once(directory.to_owned()).chain(rest)).ok()
}

/// `None` — без `XDG_RUNTIME_DIR` или `ssh` соединения не переиспользуются.
pub(crate) fn path() -> Option<&'static OsString> {
    static PATH: OnceLock<Option<OsString>> = OnceLock::new();
    let build = || {
        // сокет открывает доступ к серверу: только личный каталог сеанса
        let runtime = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?);
        shimmed(&std::env::var_os("PATH")?, &runtime.join("dimaster/ssh"))
    };
    PATH.get_or_init(build).as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    // обёртка не запускается: подставные программы ведут себя по-разному на
    // разных системах, а только что записанный файл бывает «занят»
    #[test]
    fn shim_comes_first_and_adds_master_options() {
        let root = std::env::temp_dir().join(format!("dimaster-mux-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("каталог");
        std::fs::write(root.join("ssh"), "").expect("файл");
        let own = root.join("shim");
        let path = shimmed(root.as_os_str(), &own).expect("PATH");
        let first = std::env::split_paths(&path).next();
        assert_eq!(first.as_deref(), Some(own.as_path()));
        let text = std::fs::read_to_string(own.join("ssh")).expect("обёртка");
        let real = format!(
            "exec '{}' -o ControlMaster=auto",
            root.join("ssh").display()
        );
        assert!(text.starts_with("#!/bin/sh\n") && text.contains(&real));
        assert!(text.ends_with("\"$@\"\n"));
        std::fs::remove_dir_all(root).expect("уборка");
    }
}
