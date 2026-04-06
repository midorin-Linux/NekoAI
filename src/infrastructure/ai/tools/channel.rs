use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::all::{
    Cache, ChannelId, ChannelType, EditChannel, GuildId, Http, Permissions, UserId,
};

use super::error::{DiscordToolError, require_user_permission};

// ── GetChannelInfo ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GetChannelInfoArgs {
    channel_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GetChannelInfo {
    #[serde(skip)]
    http: Option<Arc<Http>>,
}

impl GetChannelInfo {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http: Some(http) }
    }
}

impl Tool for GetChannelInfo {
    const NAME: &'static str = "get_channel_info";
    type Error = DiscordToolError;
    type Args = GetChannelInfoArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "get_channel_info".to_string(),
            description: "Get detailed information about a Discord channel by its ID.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The Discord channel ID"
                    }
                },
                "required": ["channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self.http.as_ref().ok_or_else(|| {
            DiscordToolError::new("get_channel_info", "HTTP client not available")
        })?;

        let channel = http
            .get_channel(ChannelId::new(args.channel_id))
            .await
            .map_err(|e| DiscordToolError::not_found("get_channel_info", e.to_string()))?;

        let info = match channel {
            serenity::all::Channel::Guild(gc) => {
                format!(
                    "Channel: #{} (ID: {})\nType: {:?}\nTopic: {}\nNSFW: {}\nPosition: {}",
                    gc.name,
                    gc.id,
                    gc.kind,
                    gc.topic.as_deref().unwrap_or("None"),
                    gc.nsfw,
                    gc.position,
                )
            }
            serenity::all::Channel::Private(pc) => {
                format!("Private channel with user: {}", pc.recipient.name)
            }
            _ => "Unknown channel type".to_string(),
        };

        Ok(info)
    }
}

// ── ListChannels ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListChannelsArgs {
    guild_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ListChannels {
    #[serde(skip)]
    _cache: Option<Arc<Cache>>,
    #[serde(skip)]
    http: Option<Arc<Http>>,
}

impl ListChannels {
    pub fn new(http: Arc<Http>, cache: Arc<Cache>) -> Self {
        Self {
            _cache: Some(cache),
            http: Some(http),
        }
    }
}

impl Tool for ListChannels {
    const NAME: &'static str = "list_channels";
    type Error = DiscordToolError;
    type Args = ListChannelsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "list_channels".to_string(),
            description: "List all channels in a Discord server (guild).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    }
                },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("list_channels", "HTTP client not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let channels = guild_id
            .channels(http.as_ref())
            .await
            .map_err(|e| DiscordToolError::api_error("list_channels", e.to_string()))?;

        let mut result = format!("Channels in guild {}:\n", args.guild_id);
        let mut sorted: Vec<_> = channels.values().collect();
        sorted.sort_by_key(|c| c.position);

        for ch in sorted {
            let type_str = match ch.kind {
                ChannelType::Text => "text",
                ChannelType::Voice => "voice",
                ChannelType::Category => "category",
                ChannelType::News => "news",
                ChannelType::Forum => "forum",
                ChannelType::Stage => "stage",
                _ => "other",
            };
            result.push_str(&format!(
                "- #{} (ID: {}, type: {})\n",
                ch.name, ch.id, type_str
            ));
        }

        Ok(result)
    }
}

// ── CreateChannel ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateChannelArgs {
    guild_id: u64,
    name: String,
    #[serde(default = "default_channel_type")]
    channel_type: String,
    topic: Option<String>,
    /// この操作を指示したユーザーのID（メタデータから取得）
    requesting_user_id: u64,
}

fn default_channel_type() -> String {
    "text".to_string()
}

#[derive(Serialize, Deserialize)]
pub struct CreateChannel {
    #[serde(skip)]
    http: Option<Arc<Http>>,
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl CreateChannel {
    pub fn new(http: Arc<Http>, cache: Arc<Cache>) -> Self {
        Self {
            http: Some(http),
            cache: Some(cache),
        }
    }
}

impl Tool for CreateChannel {
    const NAME: &'static str = "create_channel";
    type Error = DiscordToolError;
    type Args = CreateChannelArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "create_channel".to_string(),
            description:
                "Create a new channel in a Discord server. The requesting user must have MANAGE_CHANNELS permission."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    },
                    "name": {
                        "type": "string",
                        "description": "The name for the new channel"
                    },
                    "channel_type": {
                        "type": "string",
                        "enum": ["text", "voice", "category", "news", "forum", "stage"],
                        "description": "The type of channel to create (default: text)"
                    },
                    "topic": {
                        "type": "string",
                        "description": "The channel topic (optional, text channels only)"
                    },
                    "requesting_user_id": {
                        "type": "integer",
                        "description": "The user ID of the person who requested this action (from metadata)"
                    }
                },
                "required": ["guild_id", "name", "requesting_user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("create_channel", "HTTP client not available"))?;
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("create_channel", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let requesting_user_id = UserId::new(args.requesting_user_id);

        // MANAGE_CHANNELS 権限チェック
        require_user_permission(
            cache,
            http,
            guild_id,
            requesting_user_id,
            Permissions::MANAGE_CHANNELS,
            "create_channel",
        )
        .await?;

        let kind = match args.channel_type.as_str() {
            "voice" => ChannelType::Voice,
            "category" => ChannelType::Category,
            "news" => ChannelType::News,
            "forum" => ChannelType::Forum,
            "stage" => ChannelType::Stage,
            _ => ChannelType::Text,
        };

        let mut builder = serenity::all::CreateChannel::new(&args.name).kind(kind);
        if let Some(topic) = &args.topic {
            builder = builder.topic(topic);
        }

        let channel = guild_id
            .create_channel(http.as_ref(), builder)
            .await
            .map_err(|e| DiscordToolError::api_error("create_channel", e.to_string()))?;

        tracing::info!(
            "Channel #{} (ID: {}) created in guild {} by user {}",
            channel.name,
            channel.id,
            args.guild_id,
            args.requesting_user_id
        );

        Ok(format!(
            "Successfully created channel #{} (ID: {}, type: {:?})",
            channel.name, channel.id, channel.kind
        ))
    }
}

// ── EditChannelTool ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct EditChannelArgs {
    channel_id: u64,
    guild_id: u64,
    name: Option<String>,
    topic: Option<String>,
    /// この操作を指示したユーザーのID（メタデータから取得）
    requesting_user_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct EditChannelTool {
    #[serde(skip)]
    http: Option<Arc<Http>>,
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl EditChannelTool {
    pub fn new(http: Arc<Http>, cache: Arc<Cache>) -> Self {
        Self {
            http: Some(http),
            cache: Some(cache),
        }
    }
}

impl Tool for EditChannelTool {
    const NAME: &'static str = "edit_channel";
    type Error = DiscordToolError;
    type Args = EditChannelArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "edit_channel".to_string(),
            description:
                "Edit a Discord channel's name or topic. The requesting user must have MANAGE_CHANNELS permission."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The Discord channel ID to edit"
                    },
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    },
                    "name": {
                        "type": "string",
                        "description": "New name for the channel (optional)"
                    },
                    "topic": {
                        "type": "string",
                        "description": "New topic for the channel (optional)"
                    },
                    "requesting_user_id": {
                        "type": "integer",
                        "description": "The user ID of the person who requested this action (from metadata)"
                    }
                },
                "required": ["channel_id", "guild_id", "requesting_user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("edit_channel", "HTTP client not available"))?;
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("edit_channel", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let requesting_user_id = UserId::new(args.requesting_user_id);

        // MANAGE_CHANNELS 権限チェック
        require_user_permission(
            cache,
            http,
            guild_id,
            requesting_user_id,
            Permissions::MANAGE_CHANNELS,
            "edit_channel",
        )
        .await?;

        let channel_id = ChannelId::new(args.channel_id);

        let mut builder = EditChannel::new();
        if let Some(name) = &args.name {
            builder = builder.name(name);
        }
        if let Some(topic) = &args.topic {
            builder = builder.topic(topic);
        }

        channel_id
            .edit(http.as_ref(), builder)
            .await
            .map_err(|e| DiscordToolError::api_error("edit_channel", e.to_string()))?;

        tracing::info!(
            "Channel {} edited in guild {} by user {}",
            args.channel_id,
            args.guild_id,
            args.requesting_user_id
        );

        Ok(format!("Successfully edited channel {}", args.channel_id))
    }
}

// ── DeleteChannel ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DeleteChannelArgs {
    channel_id: u64,
    guild_id: u64,
    /// この操作を指示したユーザーのID（メタデータから取得）
    requesting_user_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct DeleteChannel {
    #[serde(skip)]
    http: Option<Arc<Http>>,
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl DeleteChannel {
    pub fn new(http: Arc<Http>, cache: Arc<Cache>) -> Self {
        Self {
            http: Some(http),
            cache: Some(cache),
        }
    }
}

impl Tool for DeleteChannel {
    const NAME: &'static str = "delete_channel";
    type Error = DiscordToolError;
    type Args = DeleteChannelArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "delete_channel".to_string(),
            description: "Delete a Discord channel. This action is irreversible. The requesting user must have MANAGE_CHANNELS permission.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The Discord channel ID to delete"
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
                "required": ["channel_id", "guild_id", "requesting_user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("delete_channel", "HTTP client not available"))?;
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("delete_channel", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let requesting_user_id = UserId::new(args.requesting_user_id);

        // MANAGE_CHANNELS 権限チェック
        require_user_permission(
            cache,
            http,
            guild_id,
            requesting_user_id,
            Permissions::MANAGE_CHANNELS,
            "delete_channel",
        )
        .await?;

        let channel_id = ChannelId::new(args.channel_id);

        channel_id
            .delete(http.as_ref())
            .await
            .map_err(|e| DiscordToolError::api_error("delete_channel", e.to_string()))?;

        tracing::info!(
            "Channel {} deleted in guild {} by user {}",
            args.channel_id,
            args.guild_id,
            args.requesting_user_id
        );

        Ok(format!("Successfully deleted channel {}", args.channel_id))
    }
}
