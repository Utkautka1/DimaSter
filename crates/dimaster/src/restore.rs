//! Связь открытого окна с хранилищем сессий.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::store::{self, Saved};
use crate::view::View;

// как часто сохранять сессию на случай сбоя
const AUTOSAVE: Duration = Duration::from_secs(30);
const MARKER: &str = "── сессия восстановлена ──";

/// Переменная, по которой новое окно узнаёт свою сессию.
pub(crate) const RESTORE_ENV: &str = "DIMASTER_RESTORE";

/// Сессия для этого окна; для остальных осиротевших открываются новые окна.
pub(crate) fn take_saved(others_running: bool) -> Option<Saved> {
    if let Some(path) = std::env::var_os(RESTORE_ENV) {
        return store::claim(&PathBuf::from(path));
    }
    if others_running {
        return None;
    }
    let orphans = store::orphans(&store::directory()?);
    let (first, rest) = orphans.split_first()?;
    rest.iter()
        .for_each(|path| open_window(None, Some(path.clone())));
    store::claim(first)
}

/// Запускает ещё одно окно в указанном каталоге.
pub(crate) fn open_window(cwd: Option<PathBuf>, restore: Option<PathBuf>) {
    let Ok(program) = std::env::current_exe() else {
        return;
    };
    let mut command = Command::new(program);
    command.stdin(Stdio::null()).env_remove("DIMASTER_SOCKET");
    if let Some(cwd) = cwd.filter(|d| d.is_dir()) {
        command.current_dir(cwd);
    }
    if let Some(path) = restore {
        command.env(RESTORE_ENV, path);
    }
    match command.spawn() {
        Ok(mut child) => drop(std::thread::spawn(move || child.wait())),
        Err(e) => log::warn!("Не удалось открыть окно: {e}"),
    }
}

impl View {
    /// Показывает сохранённую историю и запоминает каталог для shell.
    pub(crate) fn restore(&mut self, saved: Saved) {
        if !saved.lines.is_empty() {
            let text = format!("{}\r\n\x1b[2m{MARKER}\x1b[0m\r\n", saved.lines.join("\r\n"));
            self.term.advance(text.as_bytes());
        }
        self.start_directory = saved.cwd.filter(|d| d.is_dir());
    }

    /// Каталог shell: из OSC 7 или напрямую у процесса.
    pub(crate) fn shell_directory(&self) -> Option<PathBuf> {
        let reported = self.term.cwd().map(PathBuf::from);
        reported.or_else(|| {
            let pid = self.session.as_ref()?.pid();
            std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
        })
    }

    fn snapshot_session(&self) -> Saved {
        let screen = self.term.screen();
        let rows = screen.history().chain(screen.rows());
        let mut lines: Vec<String> = rows.map(|r| r.text()).filter(|l| l != MARKER).collect();
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        let skip = lines.len().saturating_sub(self.config.session_history);
        lines.drain(..skip);
        Saved {
            cwd: self.shell_directory(),
            lines,
        }
    }

    pub(crate) fn save_session(&mut self) {
        self.saved_at = Instant::now();
        // на альтернативном экране (vim, htop) истории shell не видно
        if !self.config.restore_session || self.term.is_alternate() {
            return;
        }
        let Some(directory) = store::directory() else {
            return;
        };
        if let Err(e) = store::save(&directory, &self.snapshot_session()) {
            log::warn!("Сессия не сохранена: {e}");
        }
    }

    pub(crate) fn autosave(&mut self) {
        if self.saved_at.elapsed() >= AUTOSAVE {
            self.save_session();
        }
    }

    /// `keep` — окно можно будет восстановить: закрыли последнее.
    pub(crate) fn close_session(&mut self, keep: bool) {
        if keep {
            self.save_session();
        } else if let Some(directory) = store::directory() {
            store::forget(&directory);
        }
    }
}
