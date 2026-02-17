pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod models;
pub mod presentation;

use infrastructure::{config::settings::Settings, discord::client::DiscordClient};

use anyhow::{Context, Result};

pub struct Application {
    discord_client: DiscordClient,
    settings: Settings,
}

impl Application {
    pub async fn new(settings: Settings) -> Result<Self> {
        let discord_client = DiscordClient::new(settings.discord_token.clone()).await?;

        Ok(Self {
            discord_client,
            settings,
        })
    }

    pub async fn run(self) -> Result<()> {
        let discord_client = self.discord_client.run().await.context("Failed to run Discord client")?;
        
        Ok(())
    }
}
