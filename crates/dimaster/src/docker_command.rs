//! `dimaster docker`: настройки, серверы из SSH и сам экран.

use std::error::Error;

use dimaster_config::Docker;
use dimaster_docker::App;
use dimaster_i18n::tr;

use crate::sources;

pub(crate) fn run() -> Result<(), Box<dyn Error>> {
    let config = Docker::load()?;
    if !config.settings.enabled {
        return Err(tr("экран контейнеров выключен в docker.lua").into());
    }
    let app = App::new(config, &sources::ssh_hosts());
    Ok(dimaster_docker::run(app)?)
}
