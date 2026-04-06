use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::all::{Cache, GuildId};

use super::error::DiscordToolError;

// ── GetServerInfo ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GetServerInfoArgs {
    guild_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GetServerInfo {
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl GetServerInfo {
    pub fn new(cache: Arc<Cache>) -> Self {
        Self { cache: Some(cache) }
    }
}

impl Tool for GetServerInfo {
    const NAME: &'static str = "get_server_info";
    type Error = DiscordToolError;
    type Args = GetServerInfoArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "get_server_info".to_string(),
            description: "Get detailed information about a Discord server (guild) including name, member count, creation date, boost status, etc.".to_string(),
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
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("get_server_info", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let guild = cache.guild(guild_id).ok_or_else(|| {
            DiscordToolError::not_found("get_server_info", "Guild not found in cache")
        })?;

        let owner_name = guild
            .members
            .get(&guild.owner_id)
            .map(|m| m.user.name.clone())
            .unwrap_or_else(|| format!("Unknown({})", guild.owner_id));

        let info = format!(
            "Server: {}\n\
             ID: {}\n\
             Owner: {} (ID: {})\n\
             Member Count: {}\n\
             Verification Level: {:?}\n\
             Boost Level: {:?}\n\
             Boost Count: {}\n\
             Description: {}\n\
             Icon URL: {}\n\
             Banner URL: {}\n\
             Created: {}",
            guild.name,
            guild.id,
            owner_name,
            guild.owner_id,
            guild.member_count,
            guild.verification_level,
            guild.premium_tier,
            guild.premium_subscription_count.unwrap_or(0),
            guild.description.as_deref().unwrap_or("None"),
            guild.icon_url().as_deref().unwrap_or("None"),
            guild.banner_url().as_deref().unwrap_or("None"),
            guild.id.created_at(),
        );

        Ok(info)
    }
}

// ── GetServerStats ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GetServerStatsArgs {
    guild_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GetServerStats {
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl GetServerStats {
    pub fn new(cache: Arc<Cache>) -> Self {
        Self { cache: Some(cache) }
    }
}

impl Tool for GetServerStats {
    const NAME: &'static str = "get_server_stats";
    type Error = DiscordToolError;
    type Args = GetServerStatsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "get_server_stats".to_string(),
            description: "Get server statistics: channel count, role count, emoji count, etc."
                .to_string(),
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
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("get_server_stats", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let guild = cache.guild(guild_id).ok_or_else(|| {
            DiscordToolError::not_found("get_server_stats", "Guild not found in cache")
        })?;

        let text_channels = guild
            .channels
            .values()
            .filter(|c| c.kind == serenity::all::ChannelType::Text)
            .count();
        let voice_channels = guild
            .channels
            .values()
            .filter(|c| c.kind == serenity::all::ChannelType::Voice)
            .count();
        let categories = guild
            .channels
            .values()
            .filter(|c| c.kind == serenity::all::ChannelType::Category)
            .count();

        let info = format!(
            "Server Stats for {}:\n\
             Total Channels: {}\n\
             - Text Channels: {}\n\
             - Voice Channels: {}\n\
             - Categories: {}\n\
             Roles: {}\n\
             Emojis: {}\n\
             Stickers: {}\n\
             Members (cached): {}\n\
             Online Members (approx): {}",
            guild.name,
            guild.channels.len(),
            text_channels,
            voice_channels,
            categories,
            guild.roles.len(),
            guild.emojis.len(),
            guild.stickers.len(),
            guild.members.len(),
            guild.presences.len(),
        );

        Ok(info)
    }
}
