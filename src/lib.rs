pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod models;
pub mod presentation;
pub mod shared;

use infrastructure::{ai::rig_client::RigClient, discord::client::DiscordClient};
use shared::config::Config;

use anyhow::{Context, Result};

pub struct Application {
    config: Config,
    discord_client: DiscordClient,
}

impl Application {
    pub async fn new(config: Config) -> Result<Self> {
        let rig_client = RigClient::new(
            config.ai_provider_token.clone(),
            config.provider.clone(),
            config.model.clone(),
        )
        .await?;

        let discord_client =
            DiscordClient::new(config.discord_token.clone(), config.guild_id, rig_client).await?;

        Ok(Self {
            config,
            discord_client,
        })
    }

    pub async fn run(self) -> Result<()> {
        self.discord_client
            .run()
            .await
            .context("Failed to run Discord client")?;

        Ok(())
    }
}
