use std::sync::Arc;

use agent::runtime::AgentRuntime;
use anyhow::{Context, Result};
use serenity::prelude::*;

use crate::handler::Handler;

pub struct DiscordClient {
    discord_client: Client,
}

impl DiscordClient {
    pub async fn new(
        discord_token: String,
        guild_id: u64,
        agent_runtime: AgentRuntime,
    ) -> Result<Self> {
        let intents = GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_MEMBERS
            | GatewayIntents::GUILD_VOICE_STATES
            | GatewayIntents::GUILD_PRESENCES;

        let _http = Arc::new(serenity::all::Http::new(&discord_token));

        let _shared_cache = Arc::new(serenity::all::Cache::new());

        let discord_client = Client::builder(&discord_token, intents)
            .event_handler(Handler { agent_runtime })
            .await?;

        Ok(Self { discord_client })
    }

    pub async fn run(mut self) -> Result<()> {
        self.discord_client
            .start()
            .await
            .context("Failed to start Discord client")?;

        Ok(())
    }
}
