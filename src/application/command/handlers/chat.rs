use crate::application::{command::command_registry::Context, traits::ai_client::AIClient};

#[poise::command(prefix_command, required_permissions = "ADMINISTRATOR")]
pub async fn chat(
    ctx: Context<'_>,
    #[description = "Prompt"] prompt: String,
) -> anyhow::Result<()> {
    let _typing = ctx.channel_id().start_typing(&ctx.serenity_context().http);

    let response = &ctx.data().rig_client.generate(prompt).await?;

    ctx.say(response).await?;

    Ok(())
}
