use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::all::{Cache, ChannelId, GuildId, Http};

use super::error::DiscordToolError;

// ── GetVoiceChannelInfo ─────────────────────────────────────────

#[derive(Deserialize)]
pub struct GetVoiceChannelInfoArgs {
    channel_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GetVoiceChannelInfo {
    #[serde(skip)]
    http: Option<Arc<Http>>,
}

impl GetVoiceChannelInfo {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http: Some(http) }
    }
}

impl Tool for GetVoiceChannelInfo {
    const NAME: &'static str = "get_voice_channel_info";
    type Error = DiscordToolError;
    type Args = GetVoiceChannelInfoArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "get_voice_channel_info".to_string(),
            description: "Get information about a voice channel (bitrate, user limit, etc.)."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The voice channel ID"
                    }
                },
                "required": ["channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self.http.as_ref().ok_or_else(|| {
            DiscordToolError::new("get_voice_channel_info", "HTTP client not available")
        })?;

        let channel_id = ChannelId::new(args.channel_id);
        let channel = http
            .get_channel(channel_id)
            .await
            .map_err(|e| DiscordToolError::not_found("get_voice_channel_info", e.to_string()))?;

        match channel {
            serenity::all::Channel::Guild(gc) => {
                let bitrate = gc.bitrate.unwrap_or(0);
                let user_limit = gc.user_limit.unwrap_or(0);
                let rtc_region = gc.rtc_region.as_deref().unwrap_or("auto");

                Ok(format!(
                    "Voice Channel: #{} (ID: {})\nType: {:?}\nBitrate: {} bps\nUser Limit: {}\nRTC Region: {}\nNSFW: {}",
                    gc.name, gc.id, gc.kind, bitrate, user_limit, rtc_region, gc.nsfw
                ))
            }
            _ => Err(DiscordToolError::invalid_argument(
                "get_voice_channel_info",
                "The specified channel is not a guild channel",
            )),
        }
    }
}

// ── ListVoiceMembers ────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListVoiceMembersArgs {
    guild_id: u64,
    channel_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ListVoiceMembers {
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl ListVoiceMembers {
    pub fn new(cache: Arc<Cache>) -> Self {
        Self { cache: Some(cache) }
    }
}

impl Tool for ListVoiceMembers {
    const NAME: &'static str = "list_voice_members";
    type Error = DiscordToolError;
    type Args = ListVoiceMembersArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "list_voice_members".to_string(),
            description: "List members currently in a voice channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    },
                    "channel_id": {
                        "type": "integer",
                        "description": "The voice channel ID"
                    }
                },
                "required": ["guild_id", "channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("list_voice_members", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let channel_id = ChannelId::new(args.channel_id);

        let guild = cache.guild(guild_id).ok_or_else(|| {
            DiscordToolError::not_found("list_voice_members", "Guild not found in cache")
        })?;

        let mut members_in_vc = Vec::new();
        for (user_id, voice_state) in &guild.voice_states {
            if voice_state.channel_id == Some(channel_id) {
                let display_name = guild
                    .members
                    .get(user_id)
                    .map(|m| m.nick.as_deref().unwrap_or(&m.user.name).to_string())
                    .unwrap_or_else(|| format!("Unknown({})", user_id));

                let status = format!(
                    "muted: {}, deafened: {}, streaming: {}, video: {}",
                    voice_state.mute || voice_state.self_mute,
                    voice_state.deaf || voice_state.self_deaf,
                    voice_state.self_stream.unwrap_or(false),
                    voice_state.self_video,
                );
                members_in_vc.push(format!("- {} (ID: {}) [{}]", display_name, user_id, status));
            }
        }

        if members_in_vc.is_empty() {
            Ok(format!("No members in voice channel {}", args.channel_id))
        } else {
            Ok(format!(
                "Members in voice channel {} ({} total):\n{}",
                args.channel_id,
                members_in_vc.len(),
                members_in_vc.join("\n")
            ))
        }
    }
}
