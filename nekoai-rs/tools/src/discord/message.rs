//! Discord message tools for NekoAI.
//!
//! This module provides low-level message operations plus higher-level
//! agent-friendly message workflows.

use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use serenity::{
    all::{ChannelId, EditMessage, ExecuteWebhook, GetMessages, Webhook},
    http::Http,
};

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, get_bool, get_channel_id, get_message_id, get_string, get_u8, get_u64_list,
        get_user_id, ok, parse_reaction_type, retry_discord, to_value,
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

// ===========================================================================
// High-level message workflows
// ===========================================================================

pub struct FetchReadableChatHistory {
    http: Arc<Http>,
}

pub struct SearchChannelMessages {
    http: Arc<Http>,
}

pub struct CreatePoll {
    http: Arc<Http>,
}

pub struct SendAnnouncementWithPin {
    http: Arc<Http>,
}

impl FetchReadableChatHistory {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl SearchChannelMessages {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl CreatePoll {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl SendAnnouncementWithPin {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for FetchReadableChatHistory {
    const NAME: &'static str = "fetch_readable_chat_history";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: concat!(
                "Fetch recent messages from a channel and return them as a ",
                "readable, LLM-friendly transcript. Messages are formatted as ",
                "\"[AuthorName]: message content\" with timestamps. ",
                "Use this instead of get_discord_message_history when you need ",
                "to understand the conversation flow - it saves tokens by ",
                "returning only the essential text."
            )
            .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The Discord channel ID (snowflake)."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of messages to fetch (1-100, default 20)."
                    },
                    "before": {
                        "type": "integer",
                        "description": "Fetch messages before this message ID (for pagination)."
                    }
                },
                "required": ["channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        let limit = get_u8(&args, "limit").unwrap_or(20).min(100);
        let before = get_message_id(&args, "before");

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move {
                let mut builder = GetMessages::new().limit(limit);
                if let Some(b) = before {
                    builder = builder.before(b);
                }
                channel_id.messages(&http, builder).await
            }
        })
        .await
        {
            Ok(messages) => {
                let count = messages.len();
                let mut lines: Vec<String> = Vec::with_capacity(count);

                for msg in messages.iter().rev() {
                    let author_name = msg
                        .author
                        .global_name
                        .as_deref()
                        .unwrap_or(&msg.author.name);
                    let timestamp = msg.timestamp.format("%H:%M").to_string();
                    let content = if msg.content.is_empty() {
                        if !msg.attachments.is_empty() {
                            "[attachments]".to_string()
                        } else if !msg.embeds.is_empty() {
                            "[embed]".to_string()
                        } else {
                            "[empty]".to_string()
                        }
                    } else {
                        msg.content.clone()
                    };
                    lines.push(format!("[{}] {}: {}", timestamp, author_name, content));
                }

                Ok(ok(json!({
                    "channel_id": channel_id.get(),
                    "message_count": count,
                    "transcript": lines.join("\n"),
                })))
            }
            Err(error) => Ok(err(format!("Failed to fetch message history: {error}"))),
        }
    }
}

impl Tool for SearchChannelMessages {
    const NAME: &'static str = "search_channel_messages";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: concat!(
                "Search recent messages in a channel by keyword and optionally by author. ",
                "Returns only matching messages with their IDs, author info, and content. ",
                "Useful for finding past discussions, locating specific information, ",
                "or checking if a topic has been mentioned before."
            )
            .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The Discord channel ID (snowflake)."
                    },
                    "query": {
                        "type": "string",
                        "description": "Keyword or phrase to search for (case-insensitive partial match)."
                    },
                    "author_name": {
                        "type": "string",
                        "description": "Filter by author name (partial match, case-insensitive)."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of recent messages to scan through (1-100, default 50)."
                    }
                },
                "required": ["channel_id", "query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        let Some(query) = get_string(&args, "query") else {
            return Ok(err("query is required"));
        };
        let author_name = get_string(&args, "author_name");
        let limit = get_u8(&args, "limit").unwrap_or(50).min(100);

        let query_lower = query.to_lowercase();
        let author_lower = author_name.as_ref().map(|a| a.to_lowercase());

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move {
                let builder = GetMessages::new().limit(limit);
                channel_id.messages(&http, builder).await
            }
        })
        .await
        {
            Ok(messages) => {
                let matches: Vec<Value> = messages
                    .into_iter()
                    .filter(|msg| {
                        let content_match = msg.content.to_lowercase().contains(&query_lower);
                        let author_match = match &author_lower {
                            Some(a) => {
                                msg.author.name.to_lowercase().contains(a)
                                    || msg
                                        .author
                                        .global_name
                                        .as_ref()
                                        .is_some_and(|g| g.to_lowercase().contains(a))
                            }
                            None => true,
                        };

                        content_match && author_match
                    })
                    .map(|msg| {
                        let author_name = msg
                            .author
                            .global_name
                            .as_deref()
                            .unwrap_or(&msg.author.name);
                        json!({
                            "id": msg.id.get(),
                            "author": msg.author.name,
                            "author_display": author_name,
                            "timestamp": msg.timestamp.to_string(),
                            "content": msg.content,
                        })
                    })
                    .collect();

                Ok(ok(json!({
                    "channel_id": channel_id.get(),
                    "scanned": limit,
                    "total_matches": matches.len(),
                    "matches": matches,
                })))
            }
            Err(error) => Ok(err(format!("Failed to search messages: {error}"))),
        }
    }
}

const POLL_EMOJI_NUMBERS: &[&str] = &[
    "1\u{FE0F}\u{20E3}",
    "2\u{FE0F}\u{20E3}",
    "3\u{FE0F}\u{20E3}",
    "4\u{FE0F}\u{20E3}",
    "5\u{FE0F}\u{20E3}",
    "6\u{FE0F}\u{20E3}",
    "7\u{FE0F}\u{20E3}",
    "8\u{FE0F}\u{20E3}",
    "9\u{FE0F}\u{20E3}",
    "\u{1F51F}",
];

impl Tool for CreatePoll {
    const NAME: &'static str = "create_poll";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: concat!(
                "Create a poll in a channel. Sends a formatted poll message with ",
                "the question and numbered options, then automatically adds voting ",
                "reactions (1-9 and 10) for each option. Maximum 10 options."
            )
            .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "The Discord channel ID (snowflake)." },
                    "question": { "type": "string", "description": "The poll question to ask." },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Poll options (2-10 items).",
                        "minItems": 2,
                        "maxItems": 10
                    }
                },
                "required": ["channel_id", "question", "options"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        let Some(question) = get_string(&args, "question") else {
            return Ok(err("question is required"));
        };
        let options: Vec<String> = args
            .get("options")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if options.len() < 2 {
            return Ok(err("At least 2 options are required for a poll"));
        }
        if options.len() > 10 {
            return Ok(err("Maximum 10 options allowed"));
        }

        let mut poll_text = format!("📊 **Poll: {}**\n\n", question);
        for (i, option) in options.iter().enumerate() {
            let emoji = POLL_EMOJI_NUMBERS.get(i).unwrap_or(&"❓");
            poll_text.push_str(&format!("{} {}  \n", emoji, option));
        }
        poll_text.push_str("\n---\n*React to vote!*");

        let http = self.http.clone();
        let poll_msg = match retry_discord(|| {
            let http = http.clone();
            let text = poll_text.clone();
            async move { channel_id.say(&http, &text).await }
        })
        .await
        {
            Ok(msg) => msg,
            Err(error) => return Ok(err(format!("Failed to send poll: {error}"))),
        };

        let mut failed_reactions: Vec<usize> = Vec::new();
        for (i, _) in options.iter().enumerate() {
            if let Some(emoji_str) = POLL_EMOJI_NUMBERS.get(i) {
                let Some(reaction) = parse_reaction_type(&Value::String(emoji_str.to_string()))
                else {
                    failed_reactions.push(i + 1);
                    continue;
                };
                let http = self.http.clone();
                if retry_discord(|| {
                    let http = http.clone();
                    let reaction = reaction.clone();
                    async move {
                        channel_id
                            .create_reaction(&http, poll_msg.id, reaction)
                            .await
                    }
                })
                .await
                .is_err()
                {
                    failed_reactions.push(i + 1);
                }
            }
        }

        Ok(ok(json!({
            "success": true,
            "message_id": poll_msg.id.get(),
            "channel_id": channel_id.get(),
            "question": question,
            "option_count": options.len(),
            "reactions_added": options.len() - failed_reactions.len(),
            "failed_reactions": if failed_reactions.is_empty() { Value::Null } else { json!(failed_reactions) },
        })))
    }
}

impl Tool for SendAnnouncementWithPin {
    const NAME: &'static str = "send_announcement_with_pin";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: concat!(
                "Send an important announcement to a channel and immediately pin it. ",
                "The message is formatted with an announcement header for visibility."
            )
            .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "The Discord channel ID (snowflake)." },
                    "content": { "type": "string", "description": "The announcement message content." },
                    "title": { "type": "string", "description": "Optional announcement title/header." },
                    "urgent": { "type": "boolean", "description": "If true, adds @here ping. Use sparingly and only for truly urgent announcements." }
                },
                "required": ["channel_id", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        let Some(content) = get_string(&args, "content") else {
            return Ok(err("content is required"));
        };
        let title = get_string(&args, "title");
        let urgent = get_bool(&args, "urgent").unwrap_or(false);

        let announcement = if let Some(ref t) = title {
            let mut msg = format!("📢 **{}**\n\n{}", t, content);
            if urgent {
                msg = format!("@here\n{}", msg);
            }
            msg
        } else {
            let mut msg = format!("📢 **Announcement**\n\n{}", content);
            if urgent {
                msg = format!("@here\n{}", msg);
            }
            msg
        };

        let http = self.http.clone();
        let sent_msg = match retry_discord(|| {
            let http = http.clone();
            let text = announcement.clone();
            async move { channel_id.say(&http, &text).await }
        })
        .await
        {
            Ok(msg) => msg,
            Err(error) => return Ok(err(format!("Failed to send announcement: {error}"))),
        };

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { channel_id.pin(&http, sent_msg.id).await }
        })
        .await
        {
            Ok(()) => Ok(ok(json!({
                "success": true,
                "message_id": sent_msg.id.get(),
                "channel_id": channel_id.get(),
                "title": title,
                "pinned": true,
                "urgent": urgent,
            }))),
            Err(error) => Ok(ok(json!({
                "success": true,
                "message_id": sent_msg.id.get(),
                "channel_id": channel_id.get(),
                "title": title,
                "pinned": false,
                "pin_error": error.to_string(),
                "urgent": urgent,
            }))),
        }
    }
}

pub struct SendMessageTool {
    http: Arc<Http>,
}

pub struct SearchMessages {
    inner: SearchChannelMessages,
}

pub struct BulkDeleteMessages {
    inner: BulkDeleteDiscordMessages,
}

pub struct PinMessage {
    http: Arc<Http>,
}

pub struct AddReaction {
    inner: AddDiscordReaction,
    http: Arc<Http>,
}

pub struct SendWebhookMessage {
    http: Arc<Http>,
}

impl SendMessageTool {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl SearchMessages {
    pub fn new(http: Arc<Http>) -> Self {
        Self {
            inner: SearchChannelMessages::new(http),
        }
    }
}

impl BulkDeleteMessages {
    pub fn new(http: Arc<Http>) -> Self {
        Self {
            inner: BulkDeleteDiscordMessages::new(http),
        }
    }
}

impl PinMessage {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl AddReaction {
    pub fn new(http: Arc<Http>) -> Self {
        Self {
            inner: AddDiscordReaction::new(http.clone()),
            http,
        }
    }
}

impl SendWebhookMessage {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for SendMessageTool {
    const NAME: &'static str = "send_message";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Send a message to a channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer" },
                    "content": { "type": "string" }
                },
                "required": ["channel_id", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        crate::admin_guard_channel!(&self.http, channel_id);

        let content = get_string(&args, "content")
            .or_else(|| get_string(&args, "message"))
            .unwrap_or_default();
        if content.trim().is_empty() {
            return Ok(err("content is required"));
        }

        match retry_discord(|| {
            let http = self.http.clone();
            let content = content.clone();
            async move { channel_id.say(&http, content).await }
        })
        .await
        {
            Ok(message) => Ok(ok(to_value(&message))),
            Err(error) => Ok(err(format!("Failed to send message: {error}"))),
        }
    }
}

impl Tool for SearchMessages {
    const NAME: &'static str = "search_messages";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search recent messages in a channel by keyword and optional author."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer" },
                    "query": { "type": "string" },
                    "author_name": { "type": "string" },
                    "limit": { "type": "integer" }
                },
                "required": ["channel_id", "query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.inner.call(args).await
    }
}

impl Tool for BulkDeleteMessages {
    const NAME: &'static str = "bulk_delete_messages";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Bulk delete up to 100 messages by id in a channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer" },
                    "message_ids": { "type": "array", "items": { "type": "integer" } }
                },
                "required": ["channel_id", "message_ids"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.inner.call(args).await
    }
}

impl Tool for PinMessage {
    const NAME: &'static str = "pin_message";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Pin, unpin, or list pinned messages in a channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer" },
                    "action": { "type": "string", "enum": ["pin", "unpin", "list"] },
                    "message_id": { "type": "integer" }
                },
                "required": ["channel_id", "action"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        let Some(action) = get_string(&args, "action") else {
            return Ok(err("action is required"));
        };

        if action == "list" {
            return match retry_discord(|| {
                let http = self.http.clone();
                async move { channel_id.pins(&http).await }
            })
            .await
            {
                Ok(messages) => Ok(ok(to_value(&messages))),
                Err(error) => Ok(err(format!("Failed to list pins: {error}"))),
            };
        }

        crate::admin_guard_channel!(&self.http, channel_id);
        let Some(message_id) = get_message_id(&args, "message_id") else {
            return Ok(err("message_id is required for pin/unpin"));
        };

        let result = match action.as_str() {
            "pin" => {
                retry_discord(|| {
                    let http = self.http.clone();
                    async move { channel_id.pin(&http, message_id).await }
                })
                .await
            }
            "unpin" => {
                retry_discord(|| {
                    let http = self.http.clone();
                    async move { channel_id.unpin(&http, message_id).await }
                })
                .await
            }
            _ => return Ok(err("action must be pin, unpin, or list")),
        };

        match result {
            Ok(()) => Ok(ok(json!({ "action": action, "ok": true }))),
            Err(error) => Ok(err(format!("Failed to execute action: {error}"))),
        }
    }
}

impl Tool for AddReaction {
    const NAME: &'static str = "add_reaction";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Add a reaction to a message as the bot.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer" },
                    "message_id": { "type": "integer" },
                    "emoji": { "type": "string" }
                },
                "required": ["channel_id", "message_id", "emoji"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        crate::admin_guard_channel!(&self.http, channel_id);
        self.inner.call(args).await
    }
}

impl Tool for SendWebhookMessage {
    const NAME: &'static str = "send_webhook_message";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Send a message through a Discord webhook URL.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Used for permission checks." },
                    "webhook_url": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["channel_id", "webhook_url", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        crate::admin_guard_channel!(&self.http, channel_id);
        let Some(webhook_url) = get_string(&args, "webhook_url") else {
            return Ok(err("webhook_url is required"));
        };
        let Some(content) = get_string(&args, "content") else {
            return Ok(err("content is required"));
        };

        let webhook = match Webhook::from_url(&self.http, webhook_url.as_str()).await {
            Ok(webhook) => webhook,
            Err(error) => return Ok(err(format!("Failed to resolve webhook: {error}"))),
        };

        let mut builder = ExecuteWebhook::new().content(content);
        if let Some(username) = get_string(&args, "username") {
            builder = builder.username(username);
        }
        if let Some(avatar_url) = get_string(&args, "avatar_url") {
            builder = builder.avatar_url(avatar_url);
        }
        if let Some(thread_id) = get_channel_id(&args, "thread_id") {
            builder = builder.in_thread(thread_id);
        }

        match webhook.execute(&self.http, true, builder).await {
            Ok(message) => Ok(ok(json!({
                "sent": true,
                "message": message.map(|value| to_value(&value)),
            }))),
            Err(error) => Ok(err(format!("Failed to execute webhook: {error}"))),
        }
    }
}
