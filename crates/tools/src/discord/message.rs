//! Discord message tools for NekoAI.
//!
//! This module currently provides `send_discord_message` and will host
//! additional message-related tools as they are implemented.

use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use serenity::{
    all::{ChannelId, EditMessage, GetMessages},
    http::Http,
};

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, get_channel_id, get_message_id, get_string, get_u8, get_u64_list, get_user_id, ok,
        parse_reaction_type, retry_discord, to_value,
    },
};

/// Arguments for the `send_discord_message` tool.
#[derive(Deserialize)]
pub struct SendMessageArgs {
    /// The Discord channel ID (snowflake) to send the message to.
    pub channel_id: u64,
    /// The text content of the message to send.
    pub message: String,
}

/// Output returned by the `send_discord_message` tool.
#[derive(Debug, Serialize)]
pub struct SendMessageOutput {
    /// Whether the message was sent successfully.
    pub success: bool,
    /// The ID of the sent message, if successful.
    pub message_id: Option<String>,
    /// Error description if the message could not be sent.
    pub error: Option<String>,
}

/// Tool that sends a message to a Discord channel.
///
/// The agent calls this tool with a `channel_id` and `message` string,
/// and the bot posts the message to the specified channel.
pub struct SendDiscordMessage {
    http: Arc<Http>,
}

impl SendDiscordMessage {
    /// Create a new `SendDiscordMessage` tool.
    ///
    /// - `http`: A shared serenity HTTP client (created from the bot token).
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for SendDiscordMessage {
    const NAME: &'static str = "send_discord_message";

    type Error = DiscordToolError;
    type Args = SendMessageArgs;
    type Output = SendMessageOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: concat!(
                "Send a message to a specified Discord channel. ",
                "Use this to post messages, announcements, updates, or any text ",
                "content to any channel the bot has access to. ",
                "Provide the channel ID (a numeric snowflake) and the message content."
            )
            .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "number",
                        "description": "The Discord channel ID (numeric snowflake) to send the message to"
                    },
                    "message": {
                        "type": "string",
                        "description": "The text content of the message to send"
                    }
                },
                "required": ["channel_id", "message"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let channel_id = ChannelId::new(args.channel_id);

        tracing::info!(
            target: "nekoai-tools",
            channel_id = %args.channel_id,
            message_len = args.message.len(),
            "sending Discord message via tool"
        );

        match retry_discord(|| {
            let http = self.http.clone();
            let msg = args.message.clone();
            async move { channel_id.say(&http, &msg).await }
        })
        .await
        {
            Ok(message) => {
                tracing::info!(
                    target: "nekoai-tools",
                    message_id = %message.id,
                    "Discord message sent successfully"
                );
                Ok(SendMessageOutput {
                    success: true,
                    message_id: Some(message.id.to_string()),
                    error: None,
                })
            }
            Err(e) => {
                tracing::warn!(
                    target: "nekoai-tools",
                    error = %e,
                    "failed to send Discord message"
                );
                Ok(SendMessageOutput {
                    success: false,
                    message_id: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }
}

pub struct EditDiscordMessage {
    http: Arc<Http>,
}

pub struct DeleteDiscordMessage {
    http: Arc<Http>,
}

pub struct GetDiscordMessage {
    http: Arc<Http>,
}

pub struct BulkDeleteDiscordMessages {
    http: Arc<Http>,
}

pub struct GetDiscordMessageHistory {
    http: Arc<Http>,
}

pub struct PinDiscordMessage {
    http: Arc<Http>,
}

pub struct UnpinDiscordMessage {
    http: Arc<Http>,
}

pub struct AddDiscordReaction {
    http: Arc<Http>,
}

pub struct RemoveDiscordReaction {
    http: Arc<Http>,
}

impl EditDiscordMessage {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl DeleteDiscordMessage {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl GetDiscordMessage {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl BulkDeleteDiscordMessages {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl GetDiscordMessageHistory {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl PinDiscordMessage {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl UnpinDiscordMessage {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl AddDiscordReaction {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl RemoveDiscordReaction {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for EditDiscordMessage {
    const NAME: &'static str = "edit_discord_message";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Edit a message.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id." },
                    "message_id": { "type": "integer", "description": "Message id." },
                    "content": { "type": "string", "description": "New message content." }
                },
                "required": ["channel_id", "message_id", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        crate::admin_guard_channel!(&self.http, channel_id);
        let Some(message_id) = get_message_id(&args, "message_id") else {
            return Ok(err("message_id is required"));
        };
        let Some(content) = get_string(&args, "content") else {
            return Ok(err("content is required"));
        };

        match retry_discord(|| {
            let http = self.http.clone();
            let content = content.clone();
            async move {
                let builder = EditMessage::new().content(content);
                channel_id.edit_message(&http, message_id, builder).await
            }
        })
        .await
        {
            Ok(message) => Ok(ok(to_value(&message))),
            Err(error) => Ok(err(format!("Failed to edit message: {error}"))),
        }
    }
}

impl Tool for DeleteDiscordMessage {
    const NAME: &'static str = "delete_discord_message";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Delete a message.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id." },
                    "message_id": { "type": "integer", "description": "Message id." }
                },
                "required": ["channel_id", "message_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        crate::admin_guard_channel!(&self.http, channel_id);
        let Some(message_id) = get_message_id(&args, "message_id") else {
            return Ok(err("message_id is required"));
        };

        match retry_discord(|| {
            let http = self.http.clone();
            async move { channel_id.delete_message(&http, message_id).await }
        })
        .await
        {
            Ok(()) => Ok(ok(json!({ "deleted": true }))),
            Err(error) => Ok(err(format!("Failed to delete message: {error}"))),
        }
    }
}

impl Tool for GetDiscordMessage {
    const NAME: &'static str = "get_discord_message";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get a specific message.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id." },
                    "message_id": { "type": "integer", "description": "Message id." }
                },
                "required": ["channel_id", "message_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        let Some(message_id) = get_message_id(&args, "message_id") else {
            return Ok(err("message_id is required"));
        };

        match retry_discord(|| {
            let http = self.http.clone();
            async move { channel_id.message(&http, message_id).await }
        })
        .await
        {
            Ok(message) => Ok(ok(to_value(&message))),
            Err(error) => Ok(err(format!("Failed to fetch message: {error}"))),
        }
    }
}

impl Tool for BulkDeleteDiscordMessages {
    const NAME: &'static str = "bulk_delete_discord_messages";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Bulk delete messages from a channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id." },
                    "message_ids": { "type": "array", "items": { "type": "integer" }, "description": "Message ids to delete." }
                },
                "required": ["channel_id", "message_ids"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        crate::admin_guard_channel!(&self.http, channel_id);
        let Some(message_ids) = get_u64_list(&args, "message_ids") else {
            return Ok(err("message_ids is required"));
        };

        let message_ids = message_ids
            .into_iter()
            .map(serenity::all::MessageId::new)
            .collect::<Vec<_>>();

        match retry_discord(|| {
            let http = self.http.clone();
            let ids = message_ids.clone();
            async move { channel_id.delete_messages(&http, &ids).await }
        })
        .await
        {
            Ok(()) => Ok(ok(json!({ "deleted": message_ids.len() }))),
            Err(error) => Ok(err(format!("Failed to bulk delete messages: {error}"))),
        }
    }
}

impl Tool for GetDiscordMessageHistory {
    const NAME: &'static str = "get_discord_message_history";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Fetch message history from a channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id." },
                    "limit": { "type": "integer", "description": "Max messages (1-100)." },
                    "before": { "type": "integer", "description": "Fetch messages before this message id." },
                    "after": { "type": "integer", "description": "Fetch messages after this message id." },
                    "around": { "type": "integer", "description": "Fetch messages around this message id." }
                },
                "required": ["channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };

        match retry_discord(|| {
            let http = self.http.clone();
            let (limit, before, after, around) = (
                get_u8(&args, "limit"),
                get_message_id(&args, "before"),
                get_message_id(&args, "after"),
                get_message_id(&args, "around"),
            );
            async move {
                let mut b = GetMessages::new();
                if let Some(v) = limit {
                    b = b.limit(v);
                }
                if let Some(v) = before {
                    b = b.before(v);
                }
                if let Some(v) = after {
                    b = b.after(v);
                }
                if let Some(v) = around {
                    b = b.around(v);
                }
                channel_id.messages(&http, b).await
            }
        })
        .await
        {
            Ok(messages) => Ok(ok(to_value(&messages))),
            Err(error) => Ok(err(format!("Failed to fetch message history: {error}"))),
        }
    }
}

impl Tool for PinDiscordMessage {
    const NAME: &'static str = "pin_discord_message";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Pin a message.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id." },
                    "message_id": { "type": "integer", "description": "Message id." }
                },
                "required": ["channel_id", "message_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        crate::admin_guard_channel!(&self.http, channel_id);
        let Some(message_id) = get_message_id(&args, "message_id") else {
            return Ok(err("message_id is required"));
        };

        match retry_discord(|| {
            let http = self.http.clone();
            async move { channel_id.pin(&http, message_id).await }
        })
        .await
        {
            Ok(()) => Ok(ok(json!({ "pinned": true }))),
            Err(error) => Ok(err(format!("Failed to pin message: {error}"))),
        }
    }
}

impl Tool for UnpinDiscordMessage {
    const NAME: &'static str = "unpin_discord_message";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Unpin a message.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id." },
                    "message_id": { "type": "integer", "description": "Message id." }
                },
                "required": ["channel_id", "message_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        crate::admin_guard_channel!(&self.http, channel_id);
        let Some(message_id) = get_message_id(&args, "message_id") else {
            return Ok(err("message_id is required"));
        };

        match retry_discord(|| {
            let http = self.http.clone();
            async move { channel_id.unpin(&http, message_id).await }
        })
        .await
        {
            Ok(()) => Ok(ok(json!({ "unpinned": true }))),
            Err(error) => Ok(err(format!("Failed to unpin message: {error}"))),
        }
    }
}

impl Tool for AddDiscordReaction {
    const NAME: &'static str = "add_discord_reaction";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Add a reaction to a message.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id." },
                    "message_id": { "type": "integer", "description": "Message id." },
                    "emoji": { "type": "string", "description": "Emoji to react with." }
                },
                "required": ["channel_id", "message_id", "emoji"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        let Some(message_id) = get_message_id(&args, "message_id") else {
            return Ok(err("message_id is required"));
        };
        let Some(emoji_value) = get_string(&args, "emoji") else {
            return Ok(err("emoji is required"));
        };
        let reaction = match parse_reaction_type(&Value::String(emoji_value)) {
            Some(reaction) => reaction,
            None => return Ok(err("Invalid emoji format")),
        };

        match retry_discord(|| {
            let http = self.http.clone();
            let reaction = reaction.clone();
            async move {
                channel_id
                    .create_reaction(&http, message_id, reaction)
                    .await
            }
        })
        .await
        {
            Ok(()) => Ok(ok(json!({ "reacted": true }))),
            Err(error) => Ok(err(format!("Failed to add reaction: {error}"))),
        }
    }
}

impl Tool for RemoveDiscordReaction {
    const NAME: &'static str = "remove_discord_reaction";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Remove a reaction from a message.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id." },
                    "message_id": { "type": "integer", "description": "Message id." },
                    "emoji": { "type": "string", "description": "Emoji to remove." },
                    "user_id": { "type": "integer", "description": "User id to remove reaction for." }
                },
                "required": ["channel_id", "message_id", "emoji"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        let Some(message_id) = get_message_id(&args, "message_id") else {
            return Ok(err("message_id is required"));
        };
        let Some(emoji_value) = get_string(&args, "emoji") else {
            return Ok(err("emoji is required"));
        };
        let reaction = match parse_reaction_type(&Value::String(emoji_value)) {
            Some(reaction) => reaction,
            None => return Ok(err("Invalid emoji format")),
        };
        let user_id = get_user_id(&args, "user_id");

        match retry_discord(|| {
            let http = self.http.clone();
            let reaction = reaction.clone();
            async move {
                channel_id
                    .delete_reaction(&http, message_id, user_id, reaction)
                    .await
            }
        })
        .await
        {
            Ok(()) => Ok(ok(json!({ "removed": true }))),
            Err(error) => Ok(err(format!("Failed to remove reaction: {error}"))),
        }
    }
}
