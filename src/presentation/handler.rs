use crate::presentation::events::*;

use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

pub struct Handler {}

impl Handler {}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, new_message: Message) {
        message_handler::message(ctx, new_message).await;
    }

    async fn ready(&self, _ctx: Context, data_about_bot: Ready) {
        ready_handler::ready(_ctx, data_about_bot).await;
    }
}
