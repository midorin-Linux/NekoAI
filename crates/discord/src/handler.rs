use agent::runtime::AgentRuntime;
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

pub struct Handler {
    pub agent_runtime: AgentRuntime,
    pub spinner: indicatif::ProgressBar,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, _new_message: Message) {
        // if new_message.author.bot {
        //     return;
        // }
        //
        // let (kind, thread_id) = match new_message.channel(&ctx).await {
        //     Ok(Channel::Guild(guild_channel)) => match guild_channel.kind {
        //         ChannelType::PublicThread
        //         | ChannelType::PrivateThread
        //         | ChannelType::NewsThread => (SessionKind::Thread, Some(guild_channel.id)),
        //         _ => (SessionKind::GuildChannel, None),
        //     },
        //     Ok(Channel::Private(_)) => (SessionKind::DirectMessage, None),
        //     Ok(_) => {
        //         if new_message.guild_id.is_some() {
        //             (SessionKind::GuildChannel, None)
        //         } else {
        //             (SessionKind::DirectMessage, None)
        //         }
        //     }
        //     Err(_) => {
        //         if new_message.guild_id.is_some() {
        //             (SessionKind::GuildChannel, None)
        //         } else {
        //             (SessionKind::DirectMessage, None)
        //         }
        //     }
        // };
        //
        // let session_key = SessionKey {
        //     guild_id: new_message.guild_id,
        //     channel_id: new_message.channel_id,
        //     thread_id,
        //     kind,
        // };
        //
        // let _ = new_message.channel_id.broadcast_typing(&ctx.http).await;
        //
        // match self
        //     .agent_runtime
        //     .submit(session_key, new_message.content.clone())
        //     .await
        // {
        //     Ok(response) => {
        //         if let Err(send_err) = new_message
        //             .channel_id
        //             .say(&ctx.http, response.content)
        //             .await
        //         {
        //             eprintln!("failed to send response message: {send_err}");
        //         }
        //     }
        //     Err(err) => {
        //         if let Err(send_err) = new_message.channel_id.say(&ctx.http, err.to_string()).await
        //         {
        //             eprintln!("failed to send error message: {send_err}");
        //         }
        //     }
        // };
    }

    async fn ready(&self, _ctx: Context, data_about_bot: Ready) {
        self.spinner.finish_and_clear();
        println!(
            "Discord client ready! Logged in as {}",
            data_about_bot.user.name
        );
    }
}
