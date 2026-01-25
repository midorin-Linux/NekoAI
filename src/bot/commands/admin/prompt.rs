use crate::bot::commands::commands::Context;

/// Change system prompt (Only admin)
#[poise::command(slash_command)]
pub async fn prompt(
    ctx: Context<'_>,
    #[description = "New system prompt"] new_prompt: String,
) -> anyhow::Result<()> {
    let is_admin = ctx
        .author_member()
        .await
        .map(|m| m.permissions.map_or(
            false, |p| p.contains(serenity::model::Permissions::ADMINISTRATOR))
        )
        .unwrap_or(false);

    if !is_admin {
        ctx.say("You are not admin.").await?;
        return Ok(());
    }

    let agent = &ctx.data().agent;
    agent.update_system_prompt(new_prompt.clone()).await;
    
    ctx.say(format!("System prompt has been changed to:\n{:#?}", new_prompt)).await?;
    Ok(())
}