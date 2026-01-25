use std::string::ToString;
use crate::bot::commands::commands::Context;

/// Chat with tool (Only admin)
#[poise::command(slash_command, prefix_command, required_permissions = "ADMINISTRATOR")]
pub async fn exec(
    ctx: Context<'_>,
    #[description = "New system prompt"] message: String,
) -> anyhow::Result<()> {
    ctx.defer().await?;

    let metadata = construct_metadata(&ctx).await;
    let message = format!(
        "{}\n\n<user_input>{}</user_input>", metadata, message
    );

    let response = &ctx.data().agent
        .process_message_with_tools(&ctx.author().id.to_string(), &message, ctx.serenity_context())
        .await?;

    ctx.say(response).await?;
    Ok(())
}

async fn construct_metadata(ctx: &Context<'_>) -> String {
    let mut guild_name = "DM".to_string();
    let mut channel_name = ctx.channel_id().to_string();
    let mut category_name = "None".to_string();

    if let Some(guild_id) = ctx.guild_id() {
        if let Some(guild) = ctx.cache().guild(guild_id) {
            guild_name = guild.name.clone();

            if let Some(channel) = guild.channels.get(&ctx.channel_id()) {
                channel_name = channel.name.clone();

                if let Some(parent_id) = channel.parent_id {
                    category_name = guild.channels.get(&parent_id)
                        .map(|cat| cat.name.clone())
                        .unwrap_or_else(|| "None".to_string());
                }
            }
        }
    }

    let guild_id_str = ctx.guild_id().map(|id| id.to_string()).unwrap_or_else(|| "0".to_string());

    format!(
        "<metadata>\nGuild: {} ({})\nChannel: {} > {} ({})\nUser: {} ({})\n</metadata>",
        guild_name, guild_id_str,
        category_name, channel_name, ctx.channel_id(),
        ctx.author().name, ctx.author().id
    )
}
