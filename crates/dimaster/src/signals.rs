//! Реакция окна на просьбы программ: звонок, указатель, ход выполнения.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use dimaster_grid::Policy;
use dimaster_i18n::t;
use dimaster_render::Rect;
use winit::window::{CursorIcon, UserAttentionType};

use crate::desktop;
use crate::view::View;

// сколько держится вспышка звонка
pub(crate) const FLASH: Duration = Duration::from_millis(150);
// чаще этого звонок не повторяется: `yes $'\a'` не должен плодить процессы
const QUIET: Duration = Duration::from_millis(300);
const FRAME: u32 = 4;

/// Что окно помнит о звонках.
#[derive(Debug, Default)]
pub(crate) struct Ringer {
    heard: u64,
    pub(crate) since: Option<Instant>,
}

impl View {
    pub(crate) fn policy(&self) -> Policy {
        let programs = self.config.programs;
        Policy {
            title: programs.title,
            colors: programs.colors,
            cursor_shape: programs.cursor_shape,
            pointer_shape: programs.pointer_shape,
            progress: programs.progress,
            notifications: programs.notifications,
        }
    }

    /// Замечает новые звонки в разобранном выводе.
    pub(crate) fn check_bell(&mut self) {
        let bells = self.term.bells();
        let quiet = self.ringer.since.is_some_and(|t| t.elapsed() < QUIET);
        if bells == self.ringer.heard || quiet {
            self.ringer.heard = bells;
            return;
        }
        self.ringer.heard = bells;
        self.ringer.since = Some(Instant::now());
        self.ring();
    }

    fn ring(&mut self) {
        let bell = &self.config.bell;
        if bell.attention && !self.focused {
            let kind = Some(UserAttentionType::Informational);
            self.window.request_user_attention(kind);
        }
        if bell.notify && !self.focused {
            desktop::notify("DimaSter", self.term.title());
        }
        if let Some((program, args)) = bell.command.split_first() {
            run_detached(program, args);
        }
        self.emit("bell", "");
    }

    // вспышка рамки и полоска выполнения под заголовком
    pub(crate) fn signal_rects(&self) -> Vec<Rect> {
        let mut rects = self.progress_rect().into_iter().collect::<Vec<_>>();
        let lit = self.ringer.since.is_some_and(|t| t.elapsed() < FLASH);
        if lit && self.config.bell.flash {
            let size = self.window.inner_size();
            let (w, h, color) = (size.width, size.height, self.config.bell.color);
            let sides = [
                (0, 0, w, FRAME),
                (0, h.saturating_sub(FRAME), w, FRAME),
                (0, 0, FRAME, h),
                (w.saturating_sub(FRAME), 0, FRAME, h),
            ];
            rects.extend(sides.map(|(x, y, w, h)| self.plain_rect(x, y, (w, h), color)));
        }
        rects
    }

    fn progress_rect(&self) -> Option<Rect> {
        let progress = self.term.progress()?;
        let ansi = self.config.theme.ansi;
        // красный при ошибке, жёлтый на паузе, иначе синий
        let color = match (progress.failed, progress.paused) {
            (true, _) => ansi[1],
            (_, true) => ansi[3],
            _ => ansi[4],
        };
        let width = self.window.inner_size().width * u32::from(progress.value) / 100;
        Some(self.plain_rect(0, self.content_top(), (width, 3), color))
    }

    /// Форма указателя, которую просит программа.
    pub(crate) fn program_pointer(&self) -> Option<CursorIcon> {
        self.term.pointer_shape()?.parse().ok()
    }
}

// звук и прочее запускается отдельно и не ждётся
fn run_detached(program: &str, args: &[String]) {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    match command.spawn() {
        // поток нужен только чтобы не оставить зомби
        Ok(mut child) => drop(std::thread::spawn(move || child.wait())),
        Err(e) => {
            let values = (program, e.to_string());
            log::warn!(
                "{}",
                t!(
                    "Не удалось запустить {program}: {error}",
                    program = values.0,
                    error = values.1
                )
            );
        }
    }
}
