use std::fs::File;

use anyhow::{Context, Result};
use chrono::Utc;
use tracing_appender::non_blocking;
pub use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

pub fn init_tracing(log_level: String) -> Result<WorkerGuard> {
    if let Ok(false) = std::fs::exists("logs") {
        std::fs::create_dir("logs").context("Failed to create logs directory")?;
    }

    let date = Utc::now().format("%Y-%m-%d_%H-%M-%S");
    let file =
        File::create(format!("logs/nekoai-{}.log", date)).context("Failed to create log file")?;
    let (non_blocking, guard) = non_blocking(file);

    let env_filter = EnvFilter::new(log_level);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(env_filter)
        .with_ansi(false)
        .init();

    Ok(guard)
}
