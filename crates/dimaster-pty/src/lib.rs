//! Псевдотерминал: запуск shell и обмен байтами с ним.
//!
//! Дочерний процесс должен получить новую сессию и управляющий терминал,
//! иначе не будут работать Ctrl+C, job control и SIGWINCH.

use dimaster_i18n::t;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};

/// Размер окна PTY в ячейках.
#[derive(Debug, Clone, Copy)]
pub struct WindowSize {
    pub rows: u16,
    pub cols: u16,
}

impl WindowSize {
    fn raw(self) -> libc::winsize {
        libc::winsize {
            ws_row: self.rows,
            ws_col: self.cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }
}

/// Мастер-сторона PTY и запущенный в нём процесс.
#[derive(Debug)]
pub struct Pty {
    master: File,
    child: Child,
}

impl Pty {
    /// Запускает команду с новым PTY в качестве stdin, stdout и stderr.
    pub fn spawn(mut command: Command, size: WindowSize) -> io::Result<Self> {
        let (master, slave) = open_pair(size)?;
        command
            .stdin(Stdio::from(slave.try_clone()?))
            .stdout(Stdio::from(slave.try_clone()?))
            .stderr(Stdio::from(slave));
        // SAFETY: в pre_exec вызываются только async-signal-safe функции
        unsafe { command.pre_exec(make_controlling_tty) };
        let child = command.spawn()?;
        log::info!(
            "{}",
            t!("Запущен процесс pid={pid} в PTY", pid = child.id())
        );
        Ok(Self {
            master: File::from(master),
            child,
        })
    }

    /// Сообщает новый размер; ядро само пошлёт SIGWINCH.
    pub fn resize(&self, size: WindowSize) -> io::Result<()> {
        let raw = size.raw();
        // SAFETY: дескриптор открыт, указатель живёт до конца вызова
        check(unsafe { libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &raw) })
    }

    /// Отдельный дескриптор для чтения или записи из другого потока.
    pub fn try_clone_master(&self) -> io::Result<File> {
        self.master.try_clone()
    }

    /// Номер процесса, запущенного в PTY.
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        self.child.wait()
    }
}

impl Read for Pty {
    /// Возвращает 0, когда процесс закрыл терминал (Linux сообщает это как EIO).
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.master.read(buf) {
            Err(e) if e.raw_os_error() == Some(libc::EIO) => Ok(0),
            other => other,
        }
    }
}

impl Write for Pty {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.master.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.master.flush()
    }
}

fn check(result: libc::c_int) -> io::Result<()> {
    if result == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn open_pair(size: WindowSize) -> io::Result<(OwnedFd, OwnedFd)> {
    let (mut master, mut slave) = (0, 0);
    let raw = size.raw();
    let (name, termios) = (std::ptr::null_mut(), std::ptr::null());
    // SAFETY: openpty заполняет оба дескриптора, остальные аргументы допустимо опустить
    check(unsafe { libc::openpty(&mut master, &mut slave, name, termios, &raw) })?;
    // SAFETY: дескрипторы только что созданы и больше никому не принадлежат
    let pair = unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
    // дочерний процесс не должен унаследовать лишние копии
    set_cloexec(&pair.0)?;
    set_cloexec(&pair.1)?;
    Ok(pair)
}

fn set_cloexec(fd: &OwnedFd) -> io::Result<()> {
    // SAFETY: дескриптор открыт на всё время вызова
    check(unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) })
}

fn make_controlling_tty() -> io::Result<()> {
    // SAFETY: stdin к этому моменту уже указывает на slave
    unsafe {
        check(libc::setsid())?;
        check(libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY, 0))
    }
}
