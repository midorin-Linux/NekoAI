use crate::bot::commands::commands::Context;

/// Return "Pong!"
#[poise::command(slash_command, prefix_command)]
pub async fn ping(
    ctx: Context<'_>,
) -> anyhow::Result<()> {
    ctx.say("Pong!").await?;
    Ok(())
}