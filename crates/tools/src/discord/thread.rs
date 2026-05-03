use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{Value, json};
use serenity::{all::CreateThread, http::Http};

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, get_bool, get_channel_id, get_guild_id_default, get_message_id, get_string, get_u16,
        get_user_id, ok, parse_auto_archive_duration, parse_thread_type, to_value,
    },
};

pub struct CreateDiscordThread {
    http: Arc<Http>,
}
pub struct DeleteDiscordThread {
    http: Arc<Http>,
}
pub struct GetDiscordThreadList {
    http: Arc<Http>,
}
pub struct AddDiscordThreadMember {
    http: Arc<Http>,
}

impl CreateDiscordThread {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl DeleteDiscordThread {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl GetDiscordThreadList {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl AddDiscordThreadMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for CreateDiscordThread {
    const NAME: &'static str = "create_discord_thread";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Create a thread in a channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Parent channel id." },
                    "name": { "type": "string", "description": "Thread name." },
                    "kind": { "type": "string", "description": "Thread type (public, private, news)." },
                    "auto_archive_duration": { "type": "integer", "description": "Auto archive minutes (60, 1440, 4320, 10080)." },
                    "rate_limit_per_user": { "type": "integer", "description": "Slowmode in seconds." },
                    "invitable": { "type": "boolean", "description": "Allow non-mods to invite." },
                    "message_id": { "type": "integer", "description": "Message id to start thread from." }
                },
                "required": ["channel_id", "name"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        let Some(name) = get_string(&args, "name") else {
            return Ok(err("name is required"));
        };

        let mut builder = CreateThread::new(name);
        if let Some(kind) = args.get("kind").and_then(parse_thread_type) {
            builder = builder.kind(kind);
        }
        if let Some(duration) = args
            .get("auto_archive_duration")
            .and_then(parse_auto_archive_duration)
        {
            builder = builder.auto_archive_duration(duration);
        }
        if let Some(rate_limit) = get_u16(&args, "rate_limit_per_user") {
            builder = builder.rate_limit_per_user(rate_limit);
        }
        if let Some(invitable) = get_bool(&args, "invitable") {
            builder = builder.invitable(invitable);
        }

        let message_id = get_message_id(&args, "message_id");
        let result = match message_id {
            Some(message_id) => {
                channel_id
                    .create_thread_from_message(&self.http, message_id, builder)
                    .await
            }
            None => channel_id.create_thread(&self.http, builder).await,
        };

        match result {
            Ok(thread) => Ok(ok(to_value(&thread))),
            Err(error) => Ok(err(format!("Failed to create thread: {error}"))),
        }
    }
}

impl Tool for DeleteDiscordThread {
    const NAME: &'static str = "delete_discord_thread";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Delete a thread.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": { "thread_id": { "type": "integer", "description": "Thread channel id." } },
                "required": ["thread_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(thread_id) = get_channel_id(&args, "thread_id") else {
            return Ok(err("thread_id is required"));
        };
        match thread_id.delete(&self.http).await {
            Ok(channel) => Ok(ok(to_value(&channel))),
            Err(error) => Ok(err(format!("Failed to delete thread: {error}"))),
        }
    }
}

impl Tool for GetDiscordThreadList {
    const NAME: &'static str = "get_discord_thread_list";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List active threads in a guild.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": { "guild_id": { "type": "integer", "description": "Guild id." } },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        match guild_id.get_active_threads(&self.http).await {
            Ok(threads) => Ok(ok(to_value(&threads))),
            Err(error) => Ok(err(format!("Failed to fetch threads: {error}"))),
        }
    }
}

impl Tool for AddDiscordThreadMember {
    const NAME: &'static str = "add_discord_thread_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Add a user to a thread.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "thread_id": { "type": "integer", "description": "Thread channel id." },
                    "user_id": { "type": "integer", "description": "User id." }
                },
                "required": ["thread_id", "user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(thread_id) = get_channel_id(&args, "thread_id") else {
            return Ok(err("thread_id is required"));
        };
        let Some(user_id) = get_user_id(&args, "user_id") else {
            return Ok(err("user_id is required"));
        };
        match thread_id.add_thread_member(&self.http, user_id).await {
            Ok(()) => Ok(ok(json!({ "added": true }))),
            Err(error) => Ok(err(format!("Failed to add thread member: {error}"))),
        }
    }
}
