use anyhow::{bail, Context, Result};
use tracing::info;
use tracing_appender::{non_blocking, rolling};
pub use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

pub fn init_tracing() -> Result<WorkerGuard> {
    match std::fs::exists("logs") {
        Ok(false) => std::fs::create_dir_all("logs").context("Failed to create logs directory")?,
        Ok(true) => info!("logs directory already exists"),
        _ => bail!("Failed to check logs directory existence"),
    }

    let appender = rolling::daily("logs", "nekoai.log");
    let (non_blocking, guard) = non_blocking(appender);

    dotenvy::dotenv().ok();

    let env_filter = EnvFilter::new(std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()));

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(env_filter)
        .with_ansi(false)
        .init();

    Ok(guard)
}
