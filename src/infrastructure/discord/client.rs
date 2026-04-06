use std::sync::Arc;

use anyhow::{Context, Result};
use serenity::prelude::*;

use crate::{
    application::traits::{
        ai_client::AIClient, long_term_store::LongTermStore, short_term_store::ShortTermStore,
    },
    infrastructure::ai::rig_client::RigClient,
    presentation::handler::Handler,
    shared::{
        config::{Embedding, MemoryConfig, NLP, RateLimitConfig},
        rate_limiter::RateLimiter,
    },
};

pub struct DiscordClient {
    discord_client: Client,
}

impl DiscordClient {
    pub async fn new(
        discord_token: String,
        guild_id: u64,
        nlp_api_key: String,
        embed_api_key: String,
        nlp: NLP,
        embedding: Embedding,
        memory_config: MemoryConfig,
        rate_limit_config: RateLimitConfig,
        short_term_store: Arc<dyn ShortTermStore>,
        long_term_store: Arc<dyn LongTermStore>,
    ) -> Result<Self> {
        let intents = GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILD_MEMBERS
            | GatewayIntents::GUILD_VOICE_STATES
            | GatewayIntents::GUILD_PRESENCES;

        // Http クライアントを先に作成
        let http = Arc::new(serenity::all::Http::new(&discord_token));

        // 共有キャッシュを作成
        let shared_cache = Arc::new(serenity::all::Cache::new());

        // RigClient を構築（全ツール登録済み）
        let ai_client: Arc<dyn AIClient> = Arc::new(
            RigClient::new(
                nlp_api_key,
                embed_api_key,
                nlp,
                embedding,
                http.clone(),
                shared_cache,
            )
            .await
            .context("Failed to create RigClient with tools")?,
        );

        // レートリミッターを構築
        let rate_limiter = Arc::new(RateLimiter::new(
            rate_limit_config.messages_per_minute,
            rate_limit_config.cooldown_seconds,
        ));

        let command_framework = crate::presentation::command::command_registry::command_framework(
            guild_id,
            ai_client.clone(),
            short_term_store.clone(),
            long_term_store.clone(),
            memory_config.clone(),
        )
        .await;

        let client = Client::builder(&discord_token, intents)
            .event_handler(Handler {
                ai_client,
                short_term_store,
                long_term_store,
                memory_config,
                rate_limiter,
            })
            .framework(command_framework)
            .await
            .context("Failed to create Discord client")?;

        Ok(Self {
            discord_client: client,
        })
    }

    pub async fn run(mut self) -> Result<()> {
        self.discord_client
            .start()
            .await
            .context("Failed to start Discord client")?;

        Ok(())
    }
}
