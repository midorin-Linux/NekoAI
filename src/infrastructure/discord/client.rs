use crate::presentation::handler::Handler;

use anyhow::{Context, Result};
use serenity::prelude::*;

pub struct DiscordClient {
    discord_token: String,
    guild_id: u64
}

impl DiscordClient {
    pub async fn new(discord_token: String, guild_id: u64) -> Result<Self> {
        Ok(Self { discord_token, guild_id })
    }

    pub async fn run(self) -> Result<()> {
        let intents = GatewayIntents::all(); //ToDo: 権限を絞る

        let command_framework = crate::application::command::command_registry::command_framework(self.guild_id).await;

        let mut client = Client::builder(self.discord_token, intents)
            .event_handler(Handler {})
            .framework(command_framework)
            .await
            .context("Failed to create Discord client")?;

        client
            .start()
            .await
            .context("Failed to start Discord client")?;

        Ok(())
    }
}
