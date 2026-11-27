//! Постоянный туннель к сокету Docker на сервере: `ssh -N -L`.
//!
//! Все запросы идут каналами одного соединения, и `MaxSessions` сервера на
//! такие каналы не действует (а `docker stats` открывает по каналу на каждый
//! контейнер). Если проброс запрещён или сокет лежит в другом месте, остаётся
//! обычный `ssh://` с переиспользованием соединения.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Instant;

use crate::process::capture;
use crate::timing::{get, seconds};

const REMOTE: &str = "/var/run/docker.sock";

enum Tunnel {
    Open { child: Child, socket: PathBuf },
    Failed(Instant),
}

static TUNNELS: Mutex<BTreeMap<String, Tunnel>> = Mutex::new(BTreeMap::new());
// открывает кто-то один: иначе список и статистика подключились бы дважды
static OPENING: Mutex<()> = Mutex::new(());

fn tunnels() -> MutexGuard<'static, BTreeMap<String, Tunnel>> {
    TUNNELS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Локальный сокет работающего туннеля.
pub(crate) fn socket(name: &str) -> Option<PathBuf> {
    match tunnels().get_mut(name)? {
        Tunnel::Open { child, socket } => {
            let alive = matches!(child.try_wait(), Ok(None));
            alive.then(|| socket.clone())
        }
        Tunnel::Failed(_) => None,
    }
}

// `user@host:port` — порт `ssh` принимает только отдельным ключом
fn destination(name: &str) -> (&str, Option<&str>) {
    match name.rsplit_once(':') {
        Some((host, port)) if port.parse::<u16>().is_ok() => (host, Some(port)),
        _ => (name, None),
    }
}

fn forward(name: &str, socket: &Path) -> Command {
    let mut command = Command::new("ssh");
    command.args(["-N", "-T", "-o", "ControlPath=none"]);
    command.args([
        "-o",
        "ExitOnForwardFailure=yes",
        "-o",
        "ServerAliveInterval=15",
    ]);
    command
        .arg("-L")
        .arg(format!("{}:{REMOTE}", socket.display()));
    let (host, port) = destination(name);
    if let Some(port) = port {
        command.args(["-p", port]);
    }
    command.arg("--").arg(host);
    command
        .env("SSH_ASKPASS", "false")
        .env("SSH_ASKPASS_REQUIRE", "force");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

// сокет открывает доступ к серверу: только личный каталог сеанса
fn local_socket() -> Option<PathBuf> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let runtime = PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?);
    let directory = runtime.join("dimaster");
    std::fs::create_dir_all(&directory).ok()?;
    let number = NEXT.fetch_add(1, Ordering::Relaxed);
    Some(directory.join(format!("docker-{}-{number}.sock", std::process::id())))
}

// локальный сокет появляется и при недоступном демоне: нужен настоящий ответ
fn answers(child: &mut Child, socket: &Path) -> bool {
    let deadline = Instant::now() + seconds(get().tunnel_timeout);
    while !socket.exists() {
        if !matches!(child.try_wait(), Ok(None)) || Instant::now() > deadline {
            return false;
        }
        thread::sleep(dimaster_limits::TUNNEL_POLL_MS.millis());
    }
    let mut check = Command::new("docker");
    check.arg("-H").arg(format!("unix://{}", socket.display()));
    check.args(["version", "--format", "{{.Server.Version}}"]);
    capture(check, seconds(get().tunnel_timeout)).is_ok()
}

fn open(name: &str) -> Option<Tunnel> {
    let socket = local_socket()?;
    let _ = std::fs::remove_file(&socket);
    let mut child = forward(name, &socket).spawn().ok()?;
    if answers(&mut child, &socket) {
        return Some(Tunnel::Open { child, socket });
    }
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(socket);
    None
}

/// Открывает туннель, если его нет. Долгий вызов: только из фоновых потоков.
pub(crate) fn ensure(name: &str) {
    let _one = OPENING.lock().unwrap_or_else(PoisonError::into_inner);
    let recent = matches!(tunnels().get(name), Some(Tunnel::Failed(at)) if at.elapsed() < seconds(get().tunnel_retry));
    if recent || socket(name).is_some() {
        return;
    }
    let tunnel = open(name).unwrap_or_else(|| Tunnel::Failed(Instant::now()));
    // прежний, оборвавшийся процесс нужно дождаться
    if let Some(Tunnel::Open { mut child, .. }) = tunnels().insert(name.to_owned(), tunnel) {
        let _ = child.wait();
    }
}

/// Закрывает все соединения при выходе.
pub(crate) fn close_all() {
    for tunnel in std::mem::take(&mut *tunnels()).into_values() {
        if let Tunnel::Open { mut child, socket } = tunnel {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(socket);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_is_split_from_the_name() {
        assert_eq!(destination("u@10.0.0.1:2222"), ("u@10.0.0.1", Some("2222")));
        assert_eq!(destination("home"), ("home", None));
        let command = forward("home", Path::new("/run/x.sock"));
        let args: Vec<_> = command.get_args().collect();
        assert!(args.contains(&std::ffi::OsStr::new("/run/x.sock:/var/run/docker.sock")));
        assert_eq!(args[args.len() - 2..], ["--", "home"]);
    }
}
