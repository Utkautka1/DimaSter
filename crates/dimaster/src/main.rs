//! DimaSter: без аргументов открывает окно с shell пользователя.

mod actions;
mod animate;
mod app;
mod chrome;
mod clipboard;
mod desktop;
mod fetch;
mod headless;
mod host;
mod icat;
mod input;
mod keys;
mod modes;
mod mouse;
mod pointer;
mod reload;
mod remote;
mod restore;
mod scene;
mod select;
mod session;
mod settings;
mod setup;
mod store;
mod terminfo;
mod view;

use std::error::Error;
use std::process::ExitCode;

use dimaster_config::Config;

const USAGE: &str = "использование: dimaster [-e команда...] [--headless команда...] [--setup] [icat файл...] [fetch [--logo картинка]] [ctl команда]";

fn main() -> ExitCode {
    // wgpu слишком подробен на info, поэтому по умолчанию только предупреждения
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("{e}");
            ExitCode::FAILURE
        }
    }
}

// при первом запуске перед shell показывается мастер настройки
fn first_command(config: &Config) -> Vec<String> {
    let shell = settings::shell(config);
    let missing = settings::config_path().is_some_and(|p| !p.exists());
    match std::env::current_exe()
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
    {
        Some(exe) if missing => [vec![exe, "--setup".to_owned()], shell].concat(),
        _ => shell,
    }
}

fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let config = settings::read_or_default();
    match args.split_first() {
        None => app::run(first_command(&config), config, true),
        Some((flag, command)) if flag == "-e" && !command.is_empty() => {
            app::run(command.to_vec(), config, false)
        }
        Some((flag, command)) if flag == "--headless" && !command.is_empty() => {
            println!("{}", headless::run(command, &config.term)?);
            Ok(())
        }
        Some((flag, then)) if flag == "--setup" => Ok(setup::run(then)?),
        Some((word, files)) if word == "icat" && !files.is_empty() => Ok(icat::run(files)?),
        Some((word, request)) if word == "ctl" => Ok(remote::client(request)?),
        Some((word, options)) if word == "fetch" => Ok(fetch::run(options)?),
        _ => Err(USAGE.into()),
    }
}
