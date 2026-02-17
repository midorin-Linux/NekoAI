use serenity::all::{Context, Message};

pub async fn message(ctx: Context, new_message: Message) {
    if !new_message.author.bot
        && new_message.content == "!ping"
        && let Err(e) = new_message.channel_id.say(&ctx.http, "Pong!").await
    {
        tracing::error!("Error sending message: {:?}", e);
    }
}
