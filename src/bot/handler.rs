use crate::agent::agent::Agent;
use std::sync::Arc;

use owo_colors::OwoColorize;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

pub struct Handler {
    pub allowed_user_id: Option<u64>,
    pub agent: Arc<Agent>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, new_message: Message) {
        if new_message.author.bot {
            return;
        }

        if let Some(allowed_id) = self.allowed_user_id {
            if new_message.author.id.get() != allowed_id {
                return;
            }
        }
        if new_message.content.starts_with("w!") {
            return;
        }

        if let Err(e) = new_message.channel_id.broadcast_typing(&ctx.http).await {
            tracing::error!("Error sending typing: {:?}", e);
        }

        let user_id = new_message.author.id.to_string();
        let mut response = self
            .agent
            .process_message(&user_id, &new_message.content)
            .await;

        if let Err(e) = &response {
            tracing::error!("Error calling OpenAI API: {:?}", e);
            response = self
                .agent
                .process_message_simple(&new_message.content)
                .await;
        }

        match response {
            Ok(content) => {
                if let Err(e) = new_message.channel_id.say(&ctx.http, content).await {
                    tracing::error!("Error sending message: {:?}", e);
                }
            }
            Err(e) => {
                tracing::error!("Error calling OpenAI API again: {:?}", e);
                let error_message = format!(
                    "Sorry, something went wrong. Please try again later.\nDetails: {}",
                    e
                );
                if let Err(e) = new_message.channel_id.say(&ctx.http, error_message).await {
                    tracing::error!("Error sending error message: {:?}", e);
                }
            }
        }
    }

    async fn ready(&self, _ctx: Context, data_about_bot: Ready) {
        tracing::info!("{} is connected to discord!", data_about_bot.user.name);
        println!("{} Ready!\n", "✔".green());
    }
}