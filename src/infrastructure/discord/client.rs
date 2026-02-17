use crate::presentation::discord_handler::Handler;

use anyhow::{Context, Result};
use serenity::prelude::*;

pub struct DiscordClient {
    discord_token: String,
}

impl DiscordClient {
    pub async fn new(discord_token: String) -> Result<Self> {
        Ok(Self { discord_token })
    }

    pub async fn run(self) -> Result<()> {
        let intents = GatewayIntents::all(); //ToDo: 権限を絞る

        let mut client = Client::builder(self.discord_token, intents)
            .event_handler(Handler {})
            .await
            .context("Failed to create Discord client")?;

        client
            .start()
            .await
            .context("Failed to start Discord client")?;

        Ok(())
    }
}
