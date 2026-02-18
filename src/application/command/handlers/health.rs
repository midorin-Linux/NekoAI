use crate::application::command::command_registry::Context;

#[poise::command(slash_command, prefix_command)]
pub async fn health(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    ctx.say("Ok!").await?;
    Ok(())
}