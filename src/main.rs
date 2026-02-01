mod agent;
mod bot;
mod core;
mod models;
mod services;
mod utils;

use crate::utils::{config::Config, logger::init_tracing};

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Application (Ver.0.0.1)");

    let config = Config::load()
        .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
    let _ = init_tracing(config.clone())?;

    let mut app = bot::client::App::new(config);
    app.run().await?;

    Ok(())
}