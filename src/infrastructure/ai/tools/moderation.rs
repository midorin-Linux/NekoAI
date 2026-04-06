use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::all::{
    Cache, ChannelId, CreateEmbed, CreateMessage, GuildId, Http, Permissions, Timestamp, UserId,
};

use super::error::{DiscordToolError, require_user_permission};

// ── KickMember ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct KickMemberArgs {
    guild_id: u64,
    user_id: u64,
    /// この操作を指示したユーザーのID（メタデータから取得）
    requesting_user_id: u64,
    reason: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct KickMember {
    #[serde(skip)]
    http: Option<Arc<Http>>,
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl KickMember {
    pub fn new(http: Arc<Http>, cache: Arc<Cache>) -> Self {
        Self {
            http: Some(http),
            cache: Some(cache),
        }
    }
}

impl Tool for KickMember {
    const NAME: &'static str = "kick_member";
    type Error = DiscordToolError;
    type Args = KickMemberArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "kick_member".to_string(),
            description: "Kick a member from the server. ONLY usable when the requesting user has Administrator permission. You MUST provide the requesting_user_id from the metadata.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    },
                    "user_id": {
                        "type": "integer",
                        "description": "The user ID to kick"
                    },
                    "requesting_user_id": {
                        "type": "integer",
                        "description": "The user ID of the person who requested this action (from metadata)"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Reason for the kick (optional)"
                    }
                },
                "required": ["guild_id", "user_id", "requesting_user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("kick_member", "HTTP client not available"))?;
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("kick_member", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let user_id = UserId::new(args.user_id);
        let requesting_user_id = UserId::new(args.requesting_user_id);

        // 管理者権限チェック
        require_user_permission(
            cache,
            http,
            guild_id,
            requesting_user_id,
            Permissions::ADMINISTRATOR,
            "kick_member",
        )
        .await?;

        let member = guild_id
            .member(http.as_ref(), user_id)
            .await
            .map_err(|e| DiscordToolError::not_found("kick_member", e.to_string()))?;

        let reason = args.reason.as_deref().unwrap_or("No reason provided");

        member
            .kick_with_reason(http.as_ref(), reason)
            .await
            .map_err(|e| DiscordToolError::api_error("kick_member", e.to_string()))?;

        tracing::info!(
            "Member {} kicked from guild {} by {} (reason: {})",
            args.user_id,
            args.guild_id,
            args.requesting_user_id,
            reason
        );

        Ok(format!(
            "Successfully kicked user {} from guild {}. Reason: {}",
            args.user_id, args.guild_id, reason
        ))
    }
}

// ── BanMember ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BanMemberArgs {
    guild_id: u64,
    user_id: u64,
    requesting_user_id: u64,
    reason: Option<String>,
    delete_message_days: Option<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct BanMember {
    #[serde(skip)]
    http: Option<Arc<Http>>,
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl BanMember {
    pub fn new(http: Arc<Http>, cache: Arc<Cache>) -> Self {
        Self {
            http: Some(http),
            cache: Some(cache),
        }
    }
}

impl Tool for BanMember {
    const NAME: &'static str = "ban_member";
    type Error = DiscordToolError;
    type Args = BanMemberArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "ban_member".to_string(),
            description: "Ban a member from the server. ONLY usable when the requesting user has Administrator permission. You MUST provide the requesting_user_id from the metadata.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    },
                    "user_id": {
                        "type": "integer",
                        "description": "The user ID to ban"
                    },
                    "requesting_user_id": {
                        "type": "integer",
                        "description": "The user ID of the person who requested this action (from metadata)"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Reason for the ban (optional)"
                    },
                    "delete_message_days": {
                        "type": "integer",
                        "description": "Number of days of messages to delete (0-7, default: 0)"
                    }
                },
                "required": ["guild_id", "user_id", "requesting_user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("ban_member", "HTTP client not available"))?;
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("ban_member", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let user_id = UserId::new(args.user_id);
        let requesting_user_id = UserId::new(args.requesting_user_id);

        require_user_permission(
            cache,
            http,
            guild_id,
            requesting_user_id,
            Permissions::ADMINISTRATOR,
            "ban_member",
        )
        .await?;

        let dmd = args.delete_message_days.unwrap_or(0).min(7);
        let reason = args.reason.as_deref().unwrap_or("No reason provided");

        guild_id
            .ban_with_reason(http.as_ref(), user_id, dmd, reason)
            .await
            .map_err(|e| DiscordToolError::api_error("ban_member", e.to_string()))?;

        tracing::info!(
            "Member {} banned from guild {} by {} (reason: {}, delete_days: {})",
            args.user_id,
            args.guild_id,
            args.requesting_user_id,
            reason,
            dmd
        );

        Ok(format!(
            "Successfully banned user {} from guild {}. Reason: {}. Deleted {} day(s) of messages.",
            args.user_id, args.guild_id, reason, dmd
        ))
    }
}

// ── TimeoutMember ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TimeoutMemberArgs {
    guild_id: u64,
    user_id: u64,
    requesting_user_id: u64,
    duration_seconds: u64,
    reason: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct TimeoutMember {
    #[serde(skip)]
    http: Option<Arc<Http>>,
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl TimeoutMember {
    pub fn new(http: Arc<Http>, cache: Arc<Cache>) -> Self {
        Self {
            http: Some(http),
            cache: Some(cache),
        }
    }
}

impl Tool for TimeoutMember {
    const NAME: &'static str = "timeout_member";
    type Error = DiscordToolError;
    type Args = TimeoutMemberArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "timeout_member".to_string(),
            description: "Timeout (mute) a member for a specified duration. ONLY usable when the requesting user has Administrator permission. Max duration: 28 days (2419200 seconds).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    },
                    "user_id": {
                        "type": "integer",
                        "description": "The user ID to timeout"
                    },
                    "requesting_user_id": {
                        "type": "integer",
                        "description": "The user ID of the person who requested this action (from metadata)"
                    },
                    "duration_seconds": {
                        "type": "integer",
                        "description": "Timeout duration in seconds (max: 2419200 = 28 days)"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Reason for the timeout (optional)"
                    }
                },
                "required": ["guild_id", "user_id", "requesting_user_id", "duration_seconds"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("timeout_member", "HTTP client not available"))?;
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("timeout_member", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let user_id = UserId::new(args.user_id);
        let requesting_user_id = UserId::new(args.requesting_user_id);

        require_user_permission(
            cache,
            http,
            guild_id,
            requesting_user_id,
            Permissions::ADMINISTRATOR,
            "timeout_member",
        )
        .await?;

        if args.duration_seconds == 0 || args.duration_seconds > 2_419_200 {
            return Err(DiscordToolError::invalid_argument(
                "timeout_member",
                "duration_seconds must be between 1 and 2419200 (28 days)",
            ));
        }

        let timeout_until =
            chrono::Utc::now() + chrono::Duration::seconds(args.duration_seconds as i64);
        let timestamp = Timestamp::from(timeout_until);

        let mut member = guild_id
            .member(http.as_ref(), user_id)
            .await
            .map_err(|e| DiscordToolError::not_found("timeout_member", e.to_string()))?;

        let builder =
            serenity::all::EditMember::new().disable_communication_until(timestamp.to_string());

        member
            .edit(http.as_ref(), builder)
            .await
            .map_err(|e| DiscordToolError::api_error("timeout_member", e.to_string()))?;

        let reason = args.reason.as_deref().unwrap_or("No reason provided");

        tracing::info!(
            "Member {} timed out in guild {} by {} for {}s (reason: {})",
            args.user_id,
            args.guild_id,
            args.requesting_user_id,
            args.duration_seconds,
            reason
        );

        Ok(format!(
            "Successfully timed out user {} for {} seconds. Reason: {}",
            args.user_id, args.duration_seconds, reason
        ))
    }
}

// ── WarnMember ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct WarnMemberArgs {
    guild_id: u64,
    user_id: u64,
    requesting_user_id: u64,
    channel_id: u64,
    reason: String,
}

#[derive(Serialize, Deserialize)]
pub struct WarnMember {
    #[serde(skip)]
    http: Option<Arc<Http>>,
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl WarnMember {
    pub fn new(http: Arc<Http>, cache: Arc<Cache>) -> Self {
        Self {
            http: Some(http),
            cache: Some(cache),
        }
    }
}

impl Tool for WarnMember {
    const NAME: &'static str = "warn_member";
    type Error = DiscordToolError;
    type Args = WarnMemberArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "warn_member".to_string(),
            description: "Send a warning to a member with a rich embed. ONLY usable when the requesting user has Administrator permission.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    },
                    "user_id": {
                        "type": "integer",
                        "description": "The user ID to warn"
                    },
                    "requesting_user_id": {
                        "type": "integer",
                        "description": "The user ID of the person who requested this action (from metadata)"
                    },
                    "channel_id": {
                        "type": "integer",
                        "description": "The channel ID to send the warning embed to"
                    },
                    "reason": {
                        "type": "string",
                        "description": "The reason for the warning"
                    }
                },
                "required": ["guild_id", "user_id", "requesting_user_id", "channel_id", "reason"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("warn_member", "HTTP client not available"))?;
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("warn_member", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let requesting_user_id = UserId::new(args.requesting_user_id);

        require_user_permission(
            cache,
            http,
            guild_id,
            requesting_user_id,
            Permissions::ADMINISTRATOR,
            "warn_member",
        )
        .await?;

        let channel_id = ChannelId::new(args.channel_id);

        let embed = CreateEmbed::new()
            .title("\u{26a0}\u{fe0f} Warning")
            .description(format!("<@{}> has been warned.", args.user_id))
            .field("Reason", &args.reason, false)
            .field("Warned by", format!("<@{}>", args.requesting_user_id), true)
            .colour(serenity::all::Colour::ORANGE)
            .timestamp(serenity::all::Timestamp::now());

        let message = CreateMessage::new().embed(embed);

        channel_id
            .send_message(http.as_ref(), message)
            .await
            .map_err(|e| DiscordToolError::api_error("warn_member", e.to_string()))?;

        tracing::info!(
            "Member {} warned in guild {} by {} (reason: {})",
            args.user_id,
            args.guild_id,
            args.requesting_user_id,
            args.reason
        );

        Ok(format!(
            "Successfully warned user {}. Reason: {}",
            args.user_id, args.reason
        ))
    }
}
