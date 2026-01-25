use crate::agent::agent::Agent;
use std::sync::Arc;

use owo_colors::OwoColorize;
use serenity::{
    async_trait,
    model::{
        channel::Message,
        gateway::Ready,
    },
    prelude::*,
};

pub struct Handler {
    pub allowed_user_id: Option<u64>,
    pub agent: Arc<Agent>,
}

impl Handler {
    async fn construct_metadata(&self, ctx: &Context, msg: &Message) -> String {
        let mut guild_name = "DM".to_string();
        let mut channel_name = msg.channel_id.to_string();
        let mut category_name = "None".to_string();

        if let Some(guild_id) = msg.guild_id {
            if let Some(guild) = ctx.cache.guild(guild_id) {
                guild_name = guild.name.clone();

                if let Some(channel) = guild.channels.get(&msg.channel_id) {
                    channel_name = channel.name.clone();

                    if let Some(parent_id) = channel.parent_id {
                        category_name = guild.channels.get(&parent_id)
                            .map(|cat| cat.name.clone())
                            .unwrap_or_else(|| "None".to_string());
                    }
                }
            }
        }

        let guild_id_str = msg.guild_id.map(|id| id.to_string()).unwrap_or_else(|| "0".to_string());

        format!(
            "<metadata>\nGuild: {} ({})\nChannel: {} > {} ({})\nUser: {} ({})\n</metadata>",
            guild_name, guild_id_str,
            category_name, channel_name, msg.channel_id,
            msg.author.name, msg.author.id
        )
    }

    async fn send_split_messages(&self, ctx: &Context, msg: &Message, content: &str) {
        for chunk in content.as_bytes().chunks(1900) {
            if let Ok(text) = std::str::from_utf8(chunk) {
                if let Err(e) = msg.channel_id.say(&ctx.http, text).await {
                    tracing::error!("Error sending message chunk: {:?}", e);
                }
            }
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, new_message: Message) {
        if new_message.author.bot || new_message.content.starts_with("w!") {
            return;
        }

        if let Some(allowed_id) = self.allowed_user_id {
            if new_message.author.id.get() != allowed_id {
                return;
            }
        }

        let _typing = new_message.channel_id.start_typing(&ctx.http);

        let metadata = self.construct_metadata(&ctx, &new_message).await;
        let user_prompt = format!("{}\n\n<user_input>{}</user_input>", metadata, new_message.content);

        let mut response = self
            .agent
            .process_message(&new_message.author.id.to_string(), &user_prompt)
            .await;

        if response.is_err() {
            tracing::warn!("Primary agent failed, using simple fallback.");
            response = self.agent.process_message_simple(&user_prompt).await;
        }

        match response {
            Ok(content) => {
                self.send_split_messages(&ctx, &new_message, &content).await;
            }
            Err(e) => {
                tracing::error!("Agent processing failed: {:?}", e);
                let _ = new_message.reply(&ctx.http, format!("Error processing message: {}", e)).await;
            }
        }
    }

    async fn ready(&self, _ctx: Context, data_about_bot: Ready) {
        tracing::info!("{} is connected to Discord!", data_about_bot.user.name);
        println!("{} Ready as {}!\n", "✔".green(), data_about_bot.user.name.blue());
    }
}