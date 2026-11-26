//! Учёт команд по меткам shell (OSC 133 и 633;E): результат, длительность,
//! снимок вывода и подсказка под строкой команды.

use dimaster_i18n::tr;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::terminal::State;
use crate::untrusted::clean;
use crate::{Color, PromptMark, Style};

// сколько команд и строк вывода каждой хранится в памяти окна
const HISTORY: usize = 100;
const SNAPSHOT_LINES: usize = 200;

/// Подсказки под командой; пустая строка отключает. По умолчанию выключено всё.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SmartOptions {
    pub ok: String,
    pub empty: String,
    pub errors: bool,
    /// Программы поиска: «пусто» для них значит «ничего не найдено».
    pub search_tools: Vec<String>,
}

/// Завершившаяся команда.
#[derive(Debug, Clone)]
pub struct Finished {
    pub command: String,
    pub exit: i32,
    pub duration: Duration,
    /// Последние строки вывода.
    pub output: Vec<String>,
}

#[derive(Debug)]
struct Running {
    command: String,
    // постоянный номер первой строки вывода
    line: u64,
    started: Instant,
    full_screen: bool,
}

/// Состояние учёта команд.
#[derive(Debug, Default)]
pub(crate) struct Commands {
    pub(crate) options: SmartOptions,
    typed: String,
    running: Option<Running>,
    pub(crate) history: VecDeque<Finished>,
    // сколько команд завершилось всего: по нему окно замечает новые
    pub(crate) completed: u64,
}

/// Расшифровка кода возврата.
pub fn describe_exit(code: i32) -> String {
    let text = match code {
        1 => "общая ошибка",
        2 => "ошибка (часто — неверные аргументы)",
        126 => "нет прав на запуск",
        127 => "команда не найдена",
        130 => "SIGINT — прервано",
        131 => "SIGQUIT",
        132 => "SIGILL — недопустимая инструкция",
        134 => "SIGABRT — аварийное завершение",
        135 => "SIGBUS — ошибка шины",
        136 => "SIGFPE — ошибка вычисления",
        137 => "SIGKILL — убит (возможно, нехватка памяти)",
        139 => "SIGSEGV — ошибка сегментации",
        141 => "SIGPIPE — разорван конвейер",
        143 => "SIGTERM — завершён",
        _ => "ошибка",
    };
    tr(text)
}

impl SmartOptions {
    // текст подсказки и её цвет в палитре ANSI
    fn hint(&self, command: &str, exit: i32, silent: bool) -> Option<(String, u8)> {
        let program = command.split_whitespace().next().unwrap_or_default();
        let search = self.search_tools.iter().any(|t| t == program);
        // grep без совпадений возвращает 1, но это не ошибка
        if silent && search && exit <= 1 {
            return (!self.empty.is_empty()).then(|| (tr(&self.empty), 3));
        }
        match exit {
            0 if silent && !self.ok.is_empty() => Some((self.ok.clone(), 2)),
            _ if exit != 0 && self.errors && silent => {
                Some((format!("✗ {exit}: {}", describe_exit(exit)), 1))
            }
            _ => None,
        }
    }
}

impl State {
    fn cursor_line(&self) -> u64 {
        self.screen.evicted + (self.screen.history.len() + self.cursor.row) as u64
    }

    pub(crate) fn command_typed(&mut self, text: &str) {
        self.commands.typed = clean(text);
    }

    pub(crate) fn command_started(&mut self) {
        let command = std::mem::take(&mut self.commands.typed);
        let line = self.cursor_line();
        let started = Instant::now();
        self.commands.running = Some(Running {
            command,
            line,
            started,
            full_screen: false,
        });
    }

    pub(crate) fn entered_full_screen(&mut self) {
        if let Some(running) = &mut self.commands.running {
            running.full_screen = true;
        }
    }

    // строки вывода команды, начавшейся на `from`
    fn output_since(&self, from: u64) -> Vec<String> {
        let end = self.cursor_line() + u64::from(self.cursor.col > 0);
        let start = from.max(end.saturating_sub(SNAPSHOT_LINES as u64));
        let skip = start.saturating_sub(self.screen.evicted) as usize;
        let count = end.saturating_sub(start) as usize;
        self.screen
            .history
            .iter()
            .map(|r| r.text())
            .chain(self.screen.rows.iter().map(|r| r.text()))
            .skip(skip)
            .take(count)
            .collect()
    }

    // красная метка на строке приглашения нужна полосе прокрутки
    fn mark_failed(&mut self, before: u64) {
        let at = (before.saturating_sub(self.screen.evicted)) as usize;
        let marks = self
            .screen
            .history
            .iter_mut()
            .map(|r| &mut r.mark)
            .chain(self.screen.rows.iter_mut().map(|r| &mut r.mark));
        if let Some(mark) = marks
            .take(at + 1)
            .filter(|m| **m == Some(PromptMark::Prompt))
            .last()
        {
            *mark = Some(PromptMark::Failed);
        }
    }

    /// `D` без предшествующего `C` (пустой Enter) пропускается.
    pub(crate) fn command_finished(&mut self, exit: i32) {
        let Some(running) = self.commands.running.take() else {
            return;
        };
        if self.alternate {
            return;
        }
        let output = self.output_since(running.line);
        let silent = output.iter().all(|l| l.is_empty()) && !running.full_screen;
        if exit != 0 {
            self.mark_failed(running.line);
        }
        // после `clear` курсор выше начала — подсказка была бы не к месту
        if self.cursor_line() >= running.line
            && let Some((text, color)) = self.commands.options.hint(&running.command, exit, silent)
        {
            self.print_hint(&text, color);
        }
        let duration = running.started.elapsed();
        self.remember_command(Finished {
            command: running.command,
            exit,
            duration,
            output,
        });
    }

    fn remember_command(&mut self, finished: Finished) {
        if self.commands.history.len() == HISTORY {
            self.commands.history.pop_front();
        }
        self.commands.history.push_back(finished);
        self.commands.completed += 1;
    }

    fn print_hint(&mut self, text: &str, color: u8) {
        if self.cursor.col > 0 {
            self.set_col(0);
            self.line_feed();
        }
        let style = self.cursor.style;
        self.cursor.style = Style {
            fg: Color::Indexed(color),
            ..Style::default()
        };
        text.chars()
            .filter(|c| !c.is_control())
            .for_each(|c| self.put(c));
        self.cursor.style = style;
        self.set_col(0);
        self.line_feed();
    }
}
