use std::sync::Arc;

use agent::runtime::AgentRuntime;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serenity::prelude::*;
use tracing::info;

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
        info!(guild_id, "creating discord client");
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("    {spinner} Starting discord client...")?,
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(120));

        let intents = GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_MEMBERS
            | GatewayIntents::GUILD_VOICE_STATES
            | GatewayIntents::GUILD_PRESENCES;

        let _http = Arc::new(serenity::all::Http::new(&discord_token));

        let _shared_cache = Arc::new(serenity::all::Cache::new());

        let command_framework =
            crate::command_router::command_framework(guild_id, agent_runtime.clone()).await;
        info!("discord command framework initialized");

        let discord_client = Client::builder(&discord_token, intents)
            .event_handler(Handler {
                agent_runtime,
                spinner,
            })
            .framework(command_framework)
            .await?;

        info!("discord client created");

        Ok(Self { discord_client })
    }

    pub async fn run(mut self) -> Result<()> {
        info!("starting discord event loop");
        self.discord_client
            .start()
            .await
            .context("Failed to start Discord client")?;

        info!("discord event loop finished");

        Ok(())
    }
}
