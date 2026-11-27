//! Мастер первого запуска: выполняется текстом внутри самого терминала.

use dimaster_i18n::{tr, trf};
use std::io::{self, BufRead, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use dimaster_config::config_dir;
use dimaster_config::import::{Source, detect};
use dimaster_config::{Config, File, preset, scaffold};

use crate::{integration, settings};

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
    writeln!(out, "\n{}\n", tr("Добро пожаловать в DimaSter!"))?;
    writeln!(
        out,
        "{}",
        tr("Откуда взять настройки (шрифт, цвета, отступы, курсор)?")
    )?;
    writeln!(out, "{}", tr("  0) начать с базовых"))?;
    for (index, source) in found.iter().enumerate() {
        writeln!(out, "  {}) {}", index + 1, source.name())?;
    }
    write!(out, "{}", tr("Номер или несколько через пробел [0]: "))?;
    out.flush()
}

// None — ввод закрыт: пользователь не ответил, ничего не пишем
fn ask(found: &[Source]) -> io::Result<Option<Vec<Source>>> {
    // переносить нечего — не спрашиваем: «поставил и работает»
    if found.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let (input, mut out) = (io::stdin(), io::stdout());
    loop {
        menu(found, &mut out)?;
        let mut line = String::new();
        if input.lock().read_line(&mut line)? == 0 {
            return Ok(None);
        }
        match choose(&line, found) {
            Some(chosen) => return Ok(Some(chosen)),
            None => writeln!(out, "{}", tr("Не понял ответ, попробуйте ещё раз."))?,
        }
    }
}

fn build(chosen: &[Source], config_home: &Path) -> Config {
    let mut config = preset();
    for source in chosen {
        match source.apply(config_home, &mut config) {
            Ok(()) => println!(
                "{}",
                trf("Перенесены настройки из {name}", &[("name", source.name())])
            ),
            Err(e) => {
                let values = [("name", source.name()), ("error", &e.to_string())];
                println!(
                    "{}",
                    trf("Не удалось прочитать настройки {name}: {error}", &values)
                );
            }
        }
    }
    config
}

// изменённый файл не теряется: копия уходит в backups/<время>/
fn back_up(directory: &Path, stamp: u64, file: &File) -> io::Result<()> {
    let current = directory.join(file.path);
    if std::fs::read_to_string(&current).is_ok_and(|old| old == file.content) {
        return Ok(());
    }
    let backup = directory.join(format!("backups/{stamp}")).join(file.path);
    if let Some(parent) = backup.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(current, backup).map(drop)
}

fn write_file(directory: &Path, stamp: u64, file: &File) -> io::Result<()> {
    let path = directory.join(file.path);
    if path.exists() {
        // пользовательские файлы мастер не трогает
        if file.owned_by_user {
            return Ok(());
        }
        back_up(directory, stamp, file)?;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, &file.content)
}

// init.lua пишется последним: окно перечитает конфиг, когда всё уже на месте
fn save(directory: &Path, config: &Config) -> io::Result<()> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let mut files = scaffold(config);
    files.sort_by_key(|f| f.path == "init.lua");
    for file in &files {
        write_file(directory, stamp, file)?;
    }
    let path = directory.display().to_string();
    println!(
        "{}\n",
        trf("Настройки записаны в {path}", &[("path", &path)])
    );
    Ok(())
}

// строки берутся из только что записанного конфига (`help.welcome`)
fn welcome() {
    let lines = settings::read_or_default().help.welcome;
    if !lines.is_empty() {
        let text: Vec<_> = lines.iter().map(|l| tr(l)).collect();
        println!("{}\n", text.join("\n"));
    }
}

fn wizard() -> io::Result<()> {
    let directory = config_dir().ok_or_else(|| io::Error::other(tr("не задан HOME")))?;
    let config_home = directory.parent().unwrap_or(Path::new("."));
    let found = detect(config_home);
    match ask(&found)? {
        Some(chosen) => {
            save(&directory, &build(&chosen, config_home))?;
            welcome();
            Ok(())
        }
        None => Ok(()),
    }
}

fn exec_shell(program: &str, args: &[String]) -> io::Result<()> {
    // shell и env из только что записанного конфига важнее переданных до мастера
    let config = settings::read_or_default();
    let (program, args) = match config.shell.split_first() {
        Some((head, tail)) => (head.as_str(), tail),
        None => (program, args),
    };
    let mut command = Command::new(program);
    command.envs(settings::shell_env(&config));
    if config.smart.integration && args.is_empty() {
        let injection = integration::inject(&[program.to_owned()]);
        command.args(injection.args).envs(injection.env);
    }
    command.args(args);
    Err(command.exec())
}

/// Запускает мастер, затем заменяет процесс командой `then`, если она задана.
pub(crate) fn run(then: &[String]) -> io::Result<()> {
    if let Err(e) = wizard() {
        println!(
            "{}",
            trf(
                "Мастер настройки прерван: {error}",
                &[("error", &e.to_string())]
            )
        );
    }
    let Some((program, args)) = then.split_first() else {
        return Ok(());
    };
    exec_shell(program, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_to_import_asks_nothing() {
        assert_eq!(ask(&[]).ok(), Some(Some(Vec::new())));
    }

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
