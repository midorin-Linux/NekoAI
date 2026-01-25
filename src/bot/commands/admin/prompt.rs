use crate::bot::commands::commands::Context;

/// Change system prompt (Only admin)
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn prompt(
    ctx: Context<'_>,
    #[description = "New system prompt"] new_prompt: String,
) -> anyhow::Result<()> {
    let agent = &ctx.data().agent;
    agent.update_system_prompt(new_prompt.clone()).await;
    
    ctx.say(format!("System prompt has been changed to:\n{:#?}", new_prompt)).await?;
    Ok(())
}
