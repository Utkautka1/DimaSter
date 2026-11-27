//! `dimaster view`: разбор аргументов и загрузка файлов или конвейера.

use std::error::Error;
use std::io::{IsTerminal, Read};
use std::path::PathBuf;

use dimaster_config::Viewer;
use dimaster_data::{Database, Format, Options};
use dimaster_i18n::{t, tr};
use dimaster_view::App;

#[derive(Default)]
struct Request {
    options: Options,
    files: Vec<PathBuf>,
}

fn parse(args: &[String]) -> Result<Request, String> {
    let mut request = Request::default();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-f" | "--format" => {
                let format = args.next().and_then(|n| Format::named(n));
                request.options.format = Some(format.ok_or_else(|| tr("неизвестный формат"))?);
            }
            "--separator" => {
                let byte = args
                    .next()
                    .filter(|s| s.len() == 1)
                    .map(|s| s.as_bytes()[0]);
                request.options.separator =
                    Some(byte.ok_or_else(|| tr("разделитель — один знак"))?);
            }
            "--no-header" => request.options.no_header = true,
            file => request.files.push(PathBuf::from(file)),
        }
    }
    Ok(request)
}

// без файлов читается конвейер; по умолчанию это CSV
fn load_pipe(db: &mut Database, options: &Options) -> Result<(), Box<dyn Error>> {
    if std::io::stdin().is_terminal() {
        return Err(tr("укажите файл или передайте данные по конвейеру").into());
    }
    let mut bytes = Vec::new();
    std::io::stdin().read_to_end(&mut bytes)?;
    let format = options.format.unwrap_or(Format::Csv);
    db.load_text("stdin", format, &String::from_utf8_lossy(&bytes), options)?;
    Ok(())
}

pub(crate) fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let request = parse(args)?;
    let viewer = Viewer::load()?;
    if !viewer.settings.enabled {
        return Err(tr("просмотр таблиц выключен в view.lua").into());
    }
    let mut db = Database::new()?;
    for file in &request.files {
        db.load(file, &request.options)
            .map_err(|e| t!("{path}: {error}", path = file.display(), error = e))?;
    }
    if request.files.is_empty() {
        load_pipe(&mut db, &request.options)?;
    }
    Ok(dimaster_view::run(App::new(db)?.with(viewer))?)
}
