use nekoai_domain::agent::session::SessionKey;
use tracing::{debug, error, info};

use crate::{command_router::Context, commands::utils::session_resolver};

#[poise::command(prefix_command, slash_command)]
pub async fn ask(ctx: Context<'_>, #[description = "Prompt"] prompt: String) -> anyhow::Result<()> {
    if ctx.author().bot {
        debug!(user_id = %ctx.author().id, "ignored bot invocation");
        return Ok(());
    }

    info!(
        user_id = %ctx.author().id,
        channel_id = %ctx.channel_id(),
        prompt_len = prompt.len(),
        "processing ask command"
    );

    ctx.defer().await?;

    let guild_id = ctx.guild_id();
    let channel_id = ctx.channel_id();

    let (kind, thread_id) = session_resolver(&ctx, channel_id, guild_id).await;

    let session_key = SessionKey {
        guild_id,
        channel_id,
        thread_id,
        kind,
    };

    debug!(session = %session_key.channel_id, "session key resolved");

    let reply = match ctx
        .data()
        .agent_runtime
        .submit(session_key, prompt.clone())
        .await
    {
        Ok(response) => {
            info!(
                response_len = response.content.len(),
                "agent response generated"
            );
            format!(
                "{}: {}\n\n{}: {}",
                ctx.author().name,
                prompt,
                "Assistant",
                response.content
            )
        }
        Err(err) => {
            error!(error = %err, "failed to generate agent response");
            err.to_string()
        }
    };

    let chunks = split_message(&reply);
    info!(chunk_count = chunks.len(), "sending discord reply chunks");
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

        let split_at = remaining[.. DISCORD_MAX_LENGTH]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(DISCORD_MAX_LENGTH);

        let (chunk, rest) = remaining.split_at(split_at);
        chunks.push(chunk);
        remaining = rest;
    }

    chunks
}
