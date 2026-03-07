pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod models;
pub mod presentation;
pub mod shared;

use std::sync::Arc;

use anyhow::{Context, Result};
use application::traits::{
    ai_client::AIClient, long_term_store::LongTermStore, short_term_store::ShortTermStore,
};
use infrastructure::{
    ai::rig_client::RigClient,
    discord::client::DiscordClient,
    store::{in_memory_store::InMemoryStore, vector_store::VectorStore},
};
use serenity::all::Http;
use shared::config::Config;
use tokio::time::{Duration, interval};

pub struct Application {
    discord_client: DiscordClient,
}

impl Application {
    pub async fn new(config: Config) -> Result<Self> {
        // Discord HTTP クライアントを先に生成し、AI ツールに共有する
        let discord_http = Arc::new(Http::new(&config.discord_token));

        let ai_client: Arc<dyn AIClient> = Arc::new(
            RigClient::new(
                config.nlp_token.clone(),
                config.embed_token.clone(),
                config.nlp.clone(),
                config.embedding.clone(),
                discord_http,
            )
            .await?,
        );

        let short_term_store: Arc<dyn ShortTermStore> =
            Arc::new(InMemoryStore::new(config.nlp.max_short_term_messages));
        let long_term_store: Arc<dyn LongTermStore> = Arc::new(
            VectorStore::new(&config.qdrant_url, config.embedding.dimension)
                .await
                .context("Failed to connect to Qdrant")?,
        );

        spawn_cleanup_task(long_term_store.clone());

        let discord_client = DiscordClient::new(
            config.discord_token.clone(),
            config.guild_id,
            ai_client,
            short_term_store,
            long_term_store,
        )
        .await?;

        Ok(Self { discord_client })
    }

    pub async fn run(self) -> Result<()> {
        self.discord_client
            .run()
            .await
            .context("Failed to run Discord client")?;

        Ok(())
    }
}

fn spawn_cleanup_task(long_term_store: Arc<dyn LongTermStore>) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(60 * 60)); // 1時間ごと
        loop {
            ticker.tick().await;
            if let Err(err) = long_term_store.delete_expired_midterm().await {
                tracing::warn!("Failed to cleanup expired midterm memories: {err}");
            }
        }
    });
}
