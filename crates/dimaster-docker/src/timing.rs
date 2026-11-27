//! Ожидания из `docker.lua`: задаются один раз при запуске, читаются из потоков.

use std::sync::OnceLock;
use std::time::Duration;

use dimaster_config::DockerTiming;

static TIMING: OnceLock<DockerTiming> = OnceLock::new();

pub(crate) fn set(timing: DockerTiming) {
    // повторный запуск в том же процессе оставляет первые значения
    let _ = TIMING.set(timing);
}

pub(crate) fn get() -> DockerTiming {
    TIMING.get().copied().unwrap_or_default()
}

pub(crate) fn seconds(value: u64) -> Duration {
    Duration::from_secs(value)
}
