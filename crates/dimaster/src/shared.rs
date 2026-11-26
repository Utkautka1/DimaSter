//! Терминал, общий для потока разбора вывода и потока окна.
//!
//! Поток разбора берёт блокировку на одну порцию вывода, окно — на время
//! обработки событий, и отпускает перед ожиданием и показом кадра. Мьютекс
//! «честный»: он отдаёт блокировку ждущему окну сразу после порции, поэтому
//! поток вывода не может заставить окно голодать.

use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use dimaster_grid::Terminal;
use parking_lot::lock_api::ArcMutexGuard;
use parking_lot::{FairMutex, RawFairMutex};

pub(crate) type SharedTerminal = Arc<FairMutex<Terminal>>;

/// Что поток разбора сообщает окну.
#[derive(Debug, Default)]
pub(crate) struct Signals {
    // окно уже разбужено и ещё не обработало новости
    pending: AtomicBool,
    closed: AtomicBool,
}

impl Signals {
    /// `true`, если окно нужно разбудить: повторные порции не плодят событий.
    pub(crate) fn raise(&self) -> bool {
        !self.pending.swap(true, Ordering::AcqRel)
    }

    pub(crate) fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }

    /// Принимает новости; `true` — shell закрыл терминал.
    pub(crate) fn take(&self) -> bool {
        self.pending.store(false, Ordering::Release);
        self.closed.load(Ordering::Acquire)
    }
}

/// Сторона окна. Обращение к терминалу возможно только между [`Held::acquire`]
/// и [`Held::release`].
pub(crate) struct Held {
    shared: SharedTerminal,
    guard: Option<ArcMutexGuard<RawFairMutex, Terminal>>,
}

impl Held {
    /// Новый терминал сразу захвачен окном.
    pub(crate) fn new(terminal: Terminal) -> Self {
        let shared = Arc::new(FairMutex::new(terminal));
        let guard = Some(shared.lock_arc());
        Self { shared, guard }
    }

    pub(crate) fn share(&self) -> SharedTerminal {
        self.shared.clone()
    }

    pub(crate) fn acquire(&mut self) {
        if self.guard.is_none() {
            self.guard = Some(self.shared.lock_arc());
        }
    }

    pub(crate) fn release(&mut self) {
        self.guard = None;
    }
}

// обращение без захвата — ошибка в коде окна, а не состояние программы
const NOT_HELD: &str = "терминал не захвачен окном";

impl Deref for Held {
    type Target = Terminal;

    fn deref(&self) -> &Terminal {
        self.guard.as_deref().expect(NOT_HELD)
    }
}

impl DerefMut for Held {
    fn deref_mut(&mut self) -> &mut Terminal {
        self.guard.as_deref_mut().expect(NOT_HELD)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use dimaster_grid::Size;

    use super::*;

    #[test]
    fn parser_thread_runs_only_while_window_releases() {
        let mut held = Held::new(Terminal::new(Size { rows: 2, cols: 10 }));
        let shared = held.share();
        let worker = thread::spawn(move || shared.lock().advance(b"hi"));
        assert_eq!(held.text(), "");
        held.release();
        worker.join().unwrap();
        held.acquire();
        assert_eq!(held.text(), "hi");
    }

    #[test]
    fn wakes_are_coalesced() {
        let signals = Signals::default();
        assert!(signals.raise() && !signals.raise());
        assert!(!signals.take());
        assert!(signals.raise());
        signals.close();
        assert!(signals.take());
    }
}
