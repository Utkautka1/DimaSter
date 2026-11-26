//! DimaSter: без аргументов открывает окно с shell пользователя.

mod app;
mod headless;
mod keys;
mod mouse;
mod pointer;
mod session;
mod terminfo;
mod view;

use std::error::Error;
use std::process::ExitCode;

const USAGE: &str = "использование: dimaster [-e команда...] [--headless команда...]";

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

fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    match args.split_first() {
        None => {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
            app::run(vec![shell])
        }
        Some((flag, command)) if flag == "-e" && !command.is_empty() => app::run(command.to_vec()),
        Some((flag, command)) if flag == "--headless" && !command.is_empty() => {
            println!("{}", headless::run(command)?);
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}
