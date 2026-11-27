//! DimaSter: без аргументов открывает окно с shell пользователя.

mod actions;
mod animate;
mod app;
mod autopair;
mod bar;
mod canvas;
mod chrome;
mod clipboard;
mod desktop;
mod fetch;
mod fetch_random;
mod fetch_view;
mod headless;
mod history_store;
mod host;
mod icat;
mod input;
mod integration;
mod keymap;
mod keys;
mod minimap;
mod modes;
mod mouse;
mod overlay;
mod panels;
mod pointer;
mod presentation;
mod reload;
mod remote;
mod restore;
mod scene;
mod scrollbar;
mod select;
mod sensors;
mod session;
mod settings;
mod setup;
mod shared;
mod signals;
mod smart;
mod status;
mod store;
mod syntax;
mod terminfo;
mod view;

use dimaster_i18n::tr;
use std::error::Error;
use std::process::ExitCode;

use dimaster_config::Config;

const USAGE: &str = "использование: dimaster [-e команда...] [--headless команда...] [--setup] [icat файл...] [fetch [--logo картинка]] [ctl команда] [ssh-terminfo хост]";

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
    // ошибки самого конфига показываются на языке системы
    dimaster_i18n::set_language("auto", Vec::new());
    let config = settings::read_or_default();
    settings::set_language(&config);
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
        Some((word, host)) if word == "ssh-terminfo" && !host.is_empty() => {
            Ok(terminfo::push_over_ssh(host)?)
        }
        Some((word, request)) if word == "ctl" => Ok(remote::client(request)?),
        Some((word, options)) if word == "fetch" => Ok(fetch::run(options)?),
        _ => Err(tr(USAGE).into()),
    }
}
