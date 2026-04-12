use crate::command_router::Context;
use domain::agent::session::{SessionKey, SessionKind};
use serenity::all::{Channel, ChannelType};

#[poise::command(prefix_command, slash_command)]
pub async fn ask(ctx: Context<'_>, #[description = "Prompt"] prompt: String) -> anyhow::Result<()> {
    if ctx.author().bot {
        return Ok(());
    }

    ctx.defer().await?;

    let guild_id = ctx.guild_id();
    let channel_id = ctx.channel_id();

    let (kind, thread_id) = match channel_id.to_channel(&ctx.serenity_context().http).await {
        Ok(Channel::Guild(guild_channel)) => match guild_channel.kind {
            ChannelType::PublicThread | ChannelType::PrivateThread | ChannelType::NewsThread => {
                (SessionKind::Thread, Some(guild_channel.id))
            }
            _ => (SessionKind::GuildChannel, None),
        },
        Ok(Channel::Private(_)) => (SessionKind::DirectMessage, None),
        Ok(_) => {
            if guild_id.is_some() {
                (SessionKind::GuildChannel, None)
            } else {
                (SessionKind::DirectMessage, None)
            }
        }
        Err(_) => {
            if guild_id.is_some() {
                (SessionKind::GuildChannel, None)
            } else {
                (SessionKind::DirectMessage, None)
            }
        }
    };

    let session_key = SessionKey {
        guild_id,
        channel_id,
        thread_id,
        kind,
    };

    let reply = match ctx.data().agent_runtime.submit(session_key, prompt).await {
        Ok(response) => response.content,
        Err(err) => err.to_string(),
    };

    let chunks = split_message(&reply);
    for chunk in chunks {
        ctx.say(chunk.to_string()).await?;
    }

    Ok(())
}

const DISCORD_MAX_LENGTH: usize = 2000;

pub fn split_message(text: &str) -> Vec<&str> {
    if text.len() <= DISCORD_MAX_LENGTH {
        return vec![text];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= DISCORD_MAX_LENGTH {
            chunks.push(remaining);
            break;
        }

        let split_at = remaining[..DISCORD_MAX_LENGTH]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(DISCORD_MAX_LENGTH);

        let (chunk, rest) = remaining.split_at(split_at);
        chunks.push(chunk);
        remaining = rest;
    }

    chunks
}
