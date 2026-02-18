use anyhow::{Context, Result};
use tracing::debug;

#[tokio::main]
async fn main() -> Result<()> {
    println!("NekoAI (Ver. 0.0.2-alpha)\n");

    let config = neko_ai::shared::config::Config::load().context("Failed to load config")?;

    neko_ai::shared::logger::init_tracing(&config.log_level);

    debug!("----------BEGIN SETTINGS----------");
    debug!("{:#?}", &config);
    debug!("----------END SETTINGS----------");

    neko_ai::Application::new(config)
        .await?
        .run()
        .await
        .context("Failed to run application")
}
