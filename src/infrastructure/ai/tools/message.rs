use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::all::{
    Cache, ChannelId, CreateMessage, EditMessage, GuildId, Http, MessageId, Permissions,
    ReactionType, UserId,
};

use super::error::{DiscordToolError, require_user_permission};

// ── GetMessage ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GetMessageArgs {
    channel_id: u64,
    message_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GetMessage {
    #[serde(skip)]
    http: Option<Arc<Http>>,
}

impl GetMessage {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http: Some(http) }
    }
}

impl Tool for GetMessage {
    const NAME: &'static str = "get_message";
    type Error = DiscordToolError;
    type Args = GetMessageArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "get_message".to_string(),
            description: "Get the content of a specific message by its ID.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The channel ID where the message is"
                    },
                    "message_id": {
                        "type": "integer",
                        "description": "The message ID to retrieve"
                    }
                },
                "required": ["channel_id", "message_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("get_message", "HTTP client not available"))?;

        let channel_id = ChannelId::new(args.channel_id);
        let message_id = MessageId::new(args.message_id);

        let message = channel_id
            .message(http.as_ref(), message_id)
            .await
            .map_err(|e| DiscordToolError::not_found("get_message", e.to_string()))?;

        let info = format!(
            "Message by {} (ID: {}):\nContent: {}\nTimestamp: {}\nEdited: {}\nPinned: {}\nAttachments: {}",
            message.author.name,
            message.author.id,
            message.content,
            message.timestamp,
            message
                .edited_timestamp
                .map(|t| t.to_string())
                .unwrap_or_else(|| "No".to_string()),
            message.pinned,
            message.attachments.len(),
        );

        Ok(info)
    }
}

// ── EditMessageTool ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct EditMessageArgs {
    channel_id: u64,
    message_id: u64,
    content: String,
}

#[derive(Serialize, Deserialize)]
pub struct EditMessageTool {
    #[serde(skip)]
    http: Option<Arc<Http>>,
}

impl EditMessageTool {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http: Some(http) }
    }
}

impl Tool for EditMessageTool {
    const NAME: &'static str = "edit_message";
    type Error = DiscordToolError;
    type Args = EditMessageArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "edit_message".to_string(),
            description: "Edit a message that was sent by the bot.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The channel ID where the message is"
                    },
                    "message_id": {
                        "type": "integer",
                        "description": "The message ID to edit"
                    },
                    "content": {
                        "type": "string",
                        "description": "The new content for the message"
                    }
                },
                "required": ["channel_id", "message_id", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("edit_message", "HTTP client not available"))?;

        let channel_id = ChannelId::new(args.channel_id);
        let message_id = MessageId::new(args.message_id);

        let builder = EditMessage::new().content(&args.content);

        channel_id
            .edit_message(http.as_ref(), message_id, builder)
            .await
            .map_err(|e| DiscordToolError::api_error("edit_message", e.to_string()))?;

        Ok(format!(
            "Successfully edited message {} in channel {}",
            args.message_id, args.channel_id
        ))
    }
}

// ── DeleteMessage ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DeleteMessageArgs {
    channel_id: u64,
    message_id: u64,
    guild_id: u64,
    /// この操作を指示したユーザーのID（メタデータから取得）
    requesting_user_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct DeleteMessage {
    #[serde(skip)]
    http: Option<Arc<Http>>,
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl DeleteMessage {
    pub fn new(http: Arc<Http>, cache: Arc<Cache>) -> Self {
        Self {
            http: Some(http),
            cache: Some(cache),
        }
    }
}

impl Tool for DeleteMessage {
    const NAME: &'static str = "delete_message";
    type Error = DiscordToolError;
    type Args = DeleteMessageArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "delete_message".to_string(),
            description:
                "Delete a message. The requesting user must have MANAGE_MESSAGES permission."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The channel ID where the message is"
                    },
                    "message_id": {
                        "type": "integer",
                        "description": "The message ID to delete"
                    },
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    },
                    "requesting_user_id": {
                        "type": "integer",
                        "description": "The user ID of the person who requested this action (from metadata)"
                    }
                },
                "required": ["channel_id", "message_id", "guild_id", "requesting_user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("delete_message", "HTTP client not available"))?;
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("delete_message", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let requesting_user_id = UserId::new(args.requesting_user_id);

        // MANAGE_MESSAGES 権限チェック
        require_user_permission(
            cache,
            http,
            guild_id,
            requesting_user_id,
            Permissions::MANAGE_MESSAGES,
            "delete_message",
        )
        .await?;

        let channel_id = ChannelId::new(args.channel_id);
        let message_id = MessageId::new(args.message_id);

        channel_id
            .delete_message(http.as_ref(), message_id)
            .await
            .map_err(|e| DiscordToolError::api_error("delete_message", e.to_string()))?;

        tracing::info!(
            "Message {} deleted in channel {} (guild {}) by user {}",
            args.message_id,
            args.channel_id,
            args.guild_id,
            args.requesting_user_id
        );

        Ok(format!(
            "Successfully deleted message {} in channel {}",
            args.message_id, args.channel_id
        ))
    }
}

// ── PinMessage ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PinMessageArgs {
    channel_id: u64,
    message_id: u64,
    #[serde(default = "default_pin")]
    pin: bool,
}

fn default_pin() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
pub struct PinMessage {
    #[serde(skip)]
    http: Option<Arc<Http>>,
}

impl PinMessage {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http: Some(http) }
    }
}

impl Tool for PinMessage {
    const NAME: &'static str = "pin_message";
    type Error = DiscordToolError;
    type Args = PinMessageArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "pin_message".to_string(),
            description:
                "Pin or unpin a message in a channel. Requires MANAGE_MESSAGES permission."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The channel ID"
                    },
                    "message_id": {
                        "type": "integer",
                        "description": "The message ID to pin/unpin"
                    },
                    "pin": {
                        "type": "boolean",
                        "description": "true to pin, false to unpin (default: true)"
                    }
                },
                "required": ["channel_id", "message_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("pin_message", "HTTP client not available"))?;

        let channel_id = ChannelId::new(args.channel_id);
        let message_id = MessageId::new(args.message_id);

        if args.pin {
            channel_id
                .pin(http.as_ref(), message_id)
                .await
                .map_err(|e| DiscordToolError::api_error("pin_message", e.to_string()))?;
            Ok(format!("Successfully pinned message {}", args.message_id))
        } else {
            channel_id
                .unpin(http.as_ref(), message_id)
                .await
                .map_err(|e| DiscordToolError::api_error("pin_message", e.to_string()))?;
            Ok(format!("Successfully unpinned message {}", args.message_id))
        }
    }
}

// ── AddReaction ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AddReactionArgs {
    channel_id: u64,
    message_id: u64,
    emoji: String,
}

#[derive(Serialize, Deserialize)]
pub struct AddReaction {
    #[serde(skip)]
    http: Option<Arc<Http>>,
}

impl AddReaction {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http: Some(http) }
    }
}

impl Tool for AddReaction {
    const NAME: &'static str = "add_reaction";
    type Error = DiscordToolError;
    type Args = AddReactionArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "add_reaction".to_string(),
            description: "Add a reaction emoji to a message. Use Unicode emoji (e.g. '\u{1F44D}') or custom emoji in format 'name:id'.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The channel ID"
                    },
                    "message_id": {
                        "type": "integer",
                        "description": "The message ID to react to"
                    },
                    "emoji": {
                        "type": "string",
                        "description": "The emoji to add (Unicode emoji or custom format 'name:id')"
                    }
                },
                "required": ["channel_id", "message_id", "emoji"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("add_reaction", "HTTP client not available"))?;

        let channel_id = ChannelId::new(args.channel_id);
        let message_id = MessageId::new(args.message_id);

        let reaction = ReactionType::Unicode(args.emoji.clone());

        channel_id
            .create_reaction(http.as_ref(), message_id, reaction)
            .await
            .map_err(|e| DiscordToolError::api_error("add_reaction", e.to_string()))?;

        Ok(format!(
            "Successfully added reaction '{}' to message {}",
            args.emoji, args.message_id
        ))
    }
}

// ── SendReply ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SendReplyArgs {
    channel_id: u64,
    message_id: u64,
    content: String,
}

#[derive(Serialize, Deserialize)]
pub struct SendReply {
    #[serde(skip)]
    http: Option<Arc<Http>>,
}

impl SendReply {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http: Some(http) }
    }
}

impl Tool for SendReply {
    const NAME: &'static str = "send_reply";
    type Error = DiscordToolError;
    type Args = SendReplyArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "send_reply".to_string(),
            description: "Send a reply to a specific message in a channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The channel ID"
                    },
                    "message_id": {
                        "type": "integer",
                        "description": "The message ID to reply to"
                    },
                    "content": {
                        "type": "string",
                        "description": "The reply content"
                    }
                },
                "required": ["channel_id", "message_id", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("send_reply", "HTTP client not available"))?;

        let channel_id = ChannelId::new(args.channel_id);
        let message_id = MessageId::new(args.message_id);

        let builder = CreateMessage::new()
            .content(&args.content)
            .reference_message((channel_id, message_id));

        channel_id
            .send_message(http.as_ref(), builder)
            .await
            .map_err(|e| DiscordToolError::api_error("send_reply", e.to_string()))?;

        Ok(format!(
            "Successfully replied to message {} in channel {}",
            args.message_id, args.channel_id
        ))
    }
}
