use std::sync::Arc;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde_json::{json, Value};
use serenity::all::EditMember;
use serenity::http::Http;

use crate::discord::{
    error::DiscordToolError,
    helpers::{err, get_bool, get_channel_id, get_guild_id_default, get_user_id, ok, to_value},
};

pub struct MoveDiscordMemberVoice { http: Arc<Http> }
pub struct DisconnectDiscordMemberVoice { http: Arc<Http> }
pub struct MuteDiscordMember { http: Arc<Http> }
pub struct DeafenDiscordMember { http: Arc<Http> }

impl MoveDiscordMemberVoice { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl DisconnectDiscordMemberVoice { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl MuteDiscordMember { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl DeafenDiscordMember { pub fn new(http: Arc<Http>) -> Self { Self { http } } }

impl Tool for MoveDiscordMemberVoice {
    const NAME: &'static str = "move_discord_member_voice";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Move a member to a voice channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." },
                    "channel_id": { "type": "integer", "description": "Target voice channel id." }
                },
                "required": ["guild_id", "user_id", "channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let Some(user_id) = get_user_id(&args, "user_id") else { return Ok(err("user_id is required")); };
        let Some(channel_id) = get_channel_id(&args, "channel_id") else { return Ok(err("channel_id is required")); };

        match guild_id.move_member(&self.http, user_id, channel_id).await {
            Ok(member) => Ok(ok(to_value(&member))),
            Err(error) => Ok(err(format!("Failed to move member: {error}"))),
        }
    }
}

impl Tool for DisconnectDiscordMemberVoice {
    const NAME: &'static str = "disconnect_discord_member_voice";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Disconnect a member from voice.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." }
                },
                "required": ["guild_id", "user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let Some(user_id) = get_user_id(&args, "user_id") else { return Ok(err("user_id is required")); };

        match guild_id.disconnect_member(&self.http, user_id).await {
            Ok(member) => Ok(ok(to_value(&member))),
            Err(error) => Ok(err(format!("Failed to disconnect member: {error}"))),
        }
    }
}

impl Tool for MuteDiscordMember {
    const NAME: &'static str = "mute_discord_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Server mute a member.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." },
                    "mute": { "type": "boolean", "description": "Mute flag." }
                },
                "required": ["guild_id", "user_id", "mute"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let Some(user_id) = get_user_id(&args, "user_id") else { return Ok(err("user_id is required")); };
        let Some(mute) = get_bool(&args, "mute") else { return Ok(err("mute is required")); };

        match guild_id.edit_member(&self.http, user_id, EditMember::new().mute(mute)).await {
            Ok(member) => Ok(ok(to_value(&member))),
            Err(error) => Ok(err(format!("Failed to mute member: {error}"))),
        }
    }
}

impl Tool for DeafenDiscordMember {
    const NAME: &'static str = "deafen_discord_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Server deafen a member.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." },
                    "deafen": { "type": "boolean", "description": "Deafen flag." }
                },
                "required": ["guild_id", "user_id", "deafen"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let Some(user_id) = get_user_id(&args, "user_id") else { return Ok(err("user_id is required")); };
        let Some(deafen) = get_bool(&args, "deafen") else { return Ok(err("deafen is required")); };

        match guild_id.edit_member(&self.http, user_id, EditMember::new().deafen(deafen)).await {
            Ok(member) => Ok(ok(to_value(&member))),
            Err(error) => Ok(err(format!("Failed to deafen member: {error}"))),
        }
    }
}
