use serenity::all::{Context, Ready};

pub async fn ready(_ctx: Context, data_about_bot: Ready) {
    tracing::info!("{} is connected to Discord!", data_about_bot.user.name);
}
