use std::sync::Arc;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

use crate::{
    application::traits::{
        ai_client::AIClient, long_term_store::LongTermStore, short_term_store::ShortTermStore,
    },
    presentation::events::*,
    shared::{config::MemoryConfig, rate_limiter::RateLimiter},
};

pub struct Handler {
    pub ai_client: Arc<dyn AIClient>,
    pub short_term_store: Arc<dyn ShortTermStore>,
    pub long_term_store: Arc<dyn LongTermStore>,
    pub memory_config: MemoryConfig,
    pub rate_limiter: Arc<RateLimiter>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, new_message: Message) {
        message_handler::message(
            ctx,
            new_message,
            self.ai_client.as_ref(),
            self.short_term_store.as_ref(),
            self.long_term_store.as_ref(),
            &self.memory_config,
            &self.rate_limiter,
        )
        .await;
    }

    async fn ready(&self, ctx: Context, data_about_bot: Ready) {
        ready_handler::ready(ctx, data_about_bot).await;
    }
}
