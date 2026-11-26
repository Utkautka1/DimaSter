//! Мастер первого запуска: выполняется текстом внутри самого терминала.

use std::io::{self, BufRead, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use dimaster_config::import::{Source, detect};
use dimaster_config::{Config, to_lua};

use crate::settings;

// «1 3» или «1,3» → выбранные источники в порядке ввода; пусто — базовые настройки
fn choose(answer: &str, found: &[Source]) -> Option<Vec<Source>> {
    answer
        .split([' ', ','])
        .map(str::trim)
        .filter(|w| !w.is_empty() && *w != "0")
        .map(|w| {
            w.parse::<usize>()
                .ok()
                .and_then(|n| found.get(n.checked_sub(1)?))
                .copied()
        })
        .collect()
}

fn menu(found: &[Source], out: &mut impl Write) -> io::Result<()> {
    writeln!(out, "\nДобро пожаловать в DimaSter!\n")?;
    writeln!(
        out,
        "Откуда взять настройки (шрифт, цвета, отступы, курсор)?"
    )?;
    writeln!(out, "  0) начать с базовых")?;
    for (index, source) in found.iter().enumerate() {
        writeln!(out, "  {}) {}", index + 1, source.name())?;
    }
    write!(out, "Номер или несколько через пробел [0]: ")?;
    out.flush()
}

// None — ввод закрыт: пользователь не ответил, ничего не пишем
fn ask(found: &[Source]) -> io::Result<Option<Vec<Source>>> {
    let (input, mut out) = (io::stdin(), io::stdout());
    loop {
        menu(found, &mut out)?;
        let mut line = String::new();
        if input.lock().read_line(&mut line)? == 0 {
            return Ok(None);
        }
        match choose(&line, found) {
            Some(chosen) => return Ok(Some(chosen)),
            None => writeln!(out, "Не понял ответ, попробуйте ещё раз.")?,
        }
    }
}

fn build(chosen: &[Source], config_home: &Path) -> Config {
    let mut config = Config::default();
    for source in chosen {
        match source.apply(config_home, &mut config) {
            Ok(()) => println!("Перенесены настройки из {}", source.name()),
            Err(e) => println!("Не удалось прочитать настройки {}: {e}", source.name()),
        }
    }
    config
}

// прежний конфиг не теряется, а сохраняется рядом
fn save(path: &Path, config: &Config) -> io::Result<()> {
    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    if path.exists() {
        let backup = path.with_extension("lua.bak");
        std::fs::copy(path, &backup)?;
        println!("Старый конфиг сохранён в {}", backup.display());
    }
    std::fs::write(path, to_lua(config))?;
    println!("Конфиг записан: {}\n", path.display());
    Ok(())
}

fn wizard() -> io::Result<()> {
    let path = settings::config_path().ok_or_else(|| io::Error::other("не задан HOME"))?;
    let config_home = path.ancestors().nth(2).unwrap_or(Path::new("."));
    let found = detect(config_home);
    match ask(&found)? {
        Some(chosen) => save(&path, &build(&chosen, config_home)),
        None => Ok(()),
    }
}

/// Запускает мастер, затем заменяет процесс командой `then`, если она задана.
pub(crate) fn run(then: &[String]) -> io::Result<()> {
    if let Err(e) = wizard() {
        println!("Мастер настройки прерван: {e}");
    }
    let Some((program, args)) = then.split_first() else {
        return Ok(());
    };
    // shell из только что перенесённого конфига важнее переданного
    let config = settings::read_or_default();
    let (program, args) = config.shell.split_first().unwrap_or((program, args));
    let mut command = Command::new(program);
    command.args(args);
    Err(command.exec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_answers() {
        let found = [Source::Ghostty, Source::Kitty];
        assert_eq!(choose("\n", &found), Some(vec![]));
        assert_eq!(
            choose("2, 1\n", &found),
            Some(vec![Source::Kitty, Source::Ghostty])
        );
        assert_eq!(choose("3", &found), None);
        assert_eq!(choose("abc", &found), None);
    }
}
