use std::sync::Arc;

use serenity::all::{Context, Message};

use crate::{
    application::{
        chat::chat_service::process_message,
        traits::{
            ai_client::AIClient, long_term_store::LongTermStore, short_term_store::ShortTermStore,
        },
    },
    shared::{config::MemoryConfig, discord_utils::split_message, rate_limiter::RateLimiter},
};

pub async fn message(
    ctx: Context,
    new_message: Message,
    ai_client: &dyn AIClient,
    short_term_store: &dyn ShortTermStore,
    long_term_store: &dyn LongTermStore,
    memory_config: &MemoryConfig,
    rate_limiter: &Arc<RateLimiter>,
) {
    if new_message.author.bot {
        return;
    }

    let bot_id = ctx.cache.current_user().id;
    let mentioned = new_message.mentions.iter().any(|u| u.id == bot_id);

    if !mentioned {
        return;
    }

    let user_id = new_message.author.id.get();

    // レート制限チェック
    if !rate_limiter.check_and_consume(user_id) {
        tracing::warn!(user_id, "Rate limited user");
        if let Err(e) = new_message
            .channel_id
            .say(
                &ctx.http,
                "少し待ってからもう一度試してください。(Please wait a moment before trying again.)",
            )
            .await
        {
            tracing::error!("Error sending rate limit message: {:?}", e);
        }
        return;
    }

    let message = new_message
        .content
        .replace(&format!("<@{}>", bot_id), "")
        .replace(&format!("<@!{}>", bot_id), "")
        .trim()
        .to_string();

    if message.is_empty() {
        return;
    }

    let content = construct_metadata(&ctx, &new_message, message);

    let _typing = new_message.channel_id.start_typing(&ctx.http);

    let channel_id = new_message.channel_id.get();

    let reply = match process_message(
        ai_client,
        short_term_store,
        long_term_store,
        channel_id,
        user_id,
        content,
        memory_config,
    )
    .await
    {
        Ok(response) => response,
        Err(err) => {
            tracing::error!(
                channel_id,
                user_id,
                error = %err,
                "Failed to process mention message"
            );
            err.user_facing_message().to_string()
        }
    };

    let chunks = split_message(&reply);
    for chunk in &chunks {
        if let Err(e) = new_message.channel_id.say(&ctx.http, *chunk).await {
            tracing::error!("Error sending message: {:?}", e);
            break;
        }
    }
}

fn construct_metadata(ctx: &Context, msg: &Message, content: String) -> String {
    let mut guild_name = "DM".to_string();
    let mut channel_name = msg.channel_id.to_string();
    let mut category_name = "None".to_string();

    if let Some(guild_id) = msg.guild_id
        && let Some(guild) = ctx.cache.guild(guild_id)
    {
        guild_name = guild.name.clone();

        if let Some(channel) = guild.channels.get(&msg.channel_id) {
            channel_name = channel.name.clone();

            if let Some(parent_id) = channel.parent_id {
                category_name = guild
                    .channels
                    .get(&parent_id)
                    .map(|cat| cat.name.clone())
                    .unwrap_or_else(|| "None".to_string());
            }
        }
    }

    let guild_id_str = msg
        .guild_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "0".to_string());

    format!(
        "<metadata>\nGuild: {} ({})\nChannel: {} > {} ({})\nUser: {} ({})\n</metadata>\n\n<message>{}</message>",
        guild_name,
        guild_id_str,
        category_name,
        channel_name,
        msg.channel_id,
        msg.author.name,
        msg.author.id,
        content
    )
}
