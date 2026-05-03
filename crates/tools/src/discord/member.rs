use std::sync::Arc;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde_json::{json, Value};
use serenity::all::{EditMember, RoleId, UserId};
use serenity::http::Http;

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, get_bool, get_channel_id, get_guild_id_default, get_string, get_u32, get_u64,
        get_u64_list, get_u8, get_user_id, ok, parse_timestamp, to_value,
    },
};

pub struct GetDiscordMemberList { http: Arc<Http> }
pub struct GetDiscordMemberInfo { http: Arc<Http> }
pub struct KickDiscordMember { http: Arc<Http> }
pub struct BanDiscordMember { http: Arc<Http> }
pub struct UnbanDiscordMember { http: Arc<Http> }
pub struct BulkBanDiscordMembers { http: Arc<Http> }
pub struct ModifyDiscordMember { http: Arc<Http> }
pub struct TimeoutDiscordMember { http: Arc<Http> }

impl GetDiscordMemberList { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl GetDiscordMemberInfo { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl KickDiscordMember { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl BanDiscordMember { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl UnbanDiscordMember { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl BulkBanDiscordMembers { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl ModifyDiscordMember { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl TimeoutDiscordMember { pub fn new(http: Arc<Http>) -> Self { Self { http } } }

impl Tool for GetDiscordMemberList {
    const NAME: &'static str = "get_discord_member_list";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List guild members.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "limit": { "type": "integer", "description": "Max members to return." },
                    "after": { "type": "integer", "description": "Return members after this user id." }
                },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let limit = get_u64(&args, "limit");
        let after = get_user_id(&args, "after");

        match guild_id.members(&self.http, limit, after).await {
            Ok(members) => Ok(ok(to_value(&members))),
            Err(error) => Ok(err(format!("Failed to fetch member list: {error}"))),
        }
    }
}

impl Tool for GetDiscordMemberInfo {
    const NAME: &'static str = "get_discord_member_info";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get guild member information.".to_string(),
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

        match guild_id.member(&self.http, user_id).await {
            Ok(member) => Ok(ok(to_value(&member))),
            Err(error) => Ok(err(format!("Failed to fetch member info: {error}"))),
        }
    }
}

impl Tool for KickDiscordMember {
    const NAME: &'static str = "kick_discord_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Kick a member from the guild.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." },
                    "reason": { "type": "string", "description": "Audit log reason." }
                },
                "required": ["guild_id", "user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let Some(user_id) = get_user_id(&args, "user_id") else { return Ok(err("user_id is required")); };
        let reason = get_string(&args, "reason");

        match guild_id.kick_with_reason(&self.http, user_id, reason.as_deref().unwrap_or("")).await {
            Ok(()) => Ok(ok(json!({ "kicked": true }))),
            Err(error) => Ok(err(format!("Failed to kick member: {error}"))),
        }
    }
}

impl Tool for BanDiscordMember {
    const NAME: &'static str = "ban_discord_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Ban a member from the guild.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." },
                    "delete_message_days": { "type": "integer", "description": "Delete message history in days (0-7)." },
                    "reason": { "type": "string", "description": "Audit log reason." }
                },
                "required": ["guild_id", "user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let Some(user_id) = get_user_id(&args, "user_id") else { return Ok(err("user_id is required")); };
        let delete_message_days = get_u8(&args, "delete_message_days").unwrap_or(0);
        let reason = get_string(&args, "reason");

        match guild_id.ban_with_reason(&self.http, user_id, delete_message_days, reason.as_deref().unwrap_or("")).await {
            Ok(()) => Ok(ok(json!({ "banned": true }))),
            Err(error) => Ok(err(format!("Failed to ban member: {error}"))),
        }
    }
}

impl Tool for UnbanDiscordMember {
    const NAME: &'static str = "unban_discord_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Unban a user from the guild.".to_string(),
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

        match guild_id.unban(&self.http, user_id).await {
            Ok(()) => Ok(ok(json!({ "unbanned": true }))),
            Err(error) => Ok(err(format!("Failed to unban member: {error}"))),
        }
    }
}

impl Tool for BulkBanDiscordMembers {
    const NAME: &'static str = "bulk_ban_discord_members";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Bulk ban members from the guild.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_ids": { "type": "array", "items": { "type": "integer" }, "description": "User ids to ban." },
                    "delete_message_seconds": { "type": "integer", "description": "Delete messages younger than this many seconds." },
                    "reason": { "type": "string", "description": "Audit log reason." }
                },
                "required": ["guild_id", "user_ids"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let Some(user_ids) = get_u64_list(&args, "user_ids") else { return Ok(err("user_ids is required")); };
        let delete_message_seconds = get_u32(&args, "delete_message_seconds").unwrap_or(0);
        let reason = get_string(&args, "reason");
        let user_ids = user_ids.into_iter().map(UserId::new).collect::<Vec<_>>();

        match guild_id.bulk_ban(self.http.as_ref(), &user_ids, delete_message_seconds, reason.as_deref()).await {
            Ok(result) => Ok(ok(to_value(&result))),
            Err(error) => Ok(err(format!("Failed to bulk ban members: {error}"))),
        }
    }
}

impl Tool for ModifyDiscordMember {
    const NAME: &'static str = "modify_discord_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Modify guild member settings.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." },
                    "nick": { "type": "string", "description": "Nickname." },
                    "roles": { "type": "array", "items": { "type": "integer" }, "description": "Role ids to set." },
                    "mute": { "type": "boolean", "description": "Server mute flag." },
                    "deafen": { "type": "boolean", "description": "Server deafen flag." },
                    "channel_id": { "type": "integer", "description": "Voice channel id to move into." },
                    "disconnect": { "type": "boolean", "description": "Disconnect from voice channel." },
                    "communication_disabled_until": { "type": "string", "description": "Timeout until RFC3339 timestamp." },
                    "clear_timeout": { "type": "boolean", "description": "Clear timeout." }
                },
                "required": ["guild_id", "user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let Some(user_id) = get_user_id(&args, "user_id") else { return Ok(err("user_id is required")); };

        let mut builder = EditMember::new();
        let mut changed = false;

        if let Some(nick) = get_string(&args, "nick") { builder = builder.nickname(nick); changed = true; }
        if let Some(roles) = get_u64_list(&args, "roles") {
            let role_ids: Vec<RoleId> = roles.into_iter().map(RoleId::new).collect();
            builder = builder.roles(role_ids);
            changed = true;
        }
        if let Some(mute) = get_bool(&args, "mute") { builder = builder.mute(mute); changed = true; }
        if let Some(deafen) = get_bool(&args, "deafen") { builder = builder.deafen(deafen); changed = true; }
        if let Some(channel_id) = get_channel_id(&args, "channel_id") { builder = builder.voice_channel(channel_id); changed = true; }
        if let Some(true) = get_bool(&args, "disconnect") { builder = builder.disconnect_member(); changed = true; }
        if let Some(true) = get_bool(&args, "clear_timeout") {
            builder = builder.enable_communication();
            changed = true;
        } else if let Some(until) = args.get("communication_disabled_until").and_then(parse_timestamp) {
            builder = builder.disable_communication_until_datetime(until);
            changed = true;
        }

        if !changed { return Ok(err("No member fields provided to modify")); }

        match guild_id.edit_member(&self.http, user_id, builder).await {
            Ok(member) => Ok(ok(to_value(&member))),
            Err(error) => Ok(err(format!("Failed to modify member: {error}"))),
        }
    }
}

impl Tool for TimeoutDiscordMember {
    const NAME: &'static str = "timeout_discord_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Timeout or clear timeout for a member.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." },
                    "until": { "type": "string", "description": "RFC3339 timestamp to timeout until." },
                    "clear": { "type": "boolean", "description": "Clear timeout." }
                },
                "required": ["guild_id", "user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let Some(user_id) = get_user_id(&args, "user_id") else { return Ok(err("user_id is required")); };

        let builder = if let Some(true) = get_bool(&args, "clear") {
            EditMember::new().enable_communication()
        } else if let Some(until) = args.get("until").and_then(parse_timestamp) {
            EditMember::new().disable_communication_until_datetime(until)
        } else {
            return Ok(err("Either 'until' or 'clear' is required"));
        };

        match guild_id.edit_member(&self.http, user_id, builder).await {
            Ok(member) => Ok(ok(to_value(&member))),
            Err(error) => Ok(err(format!("Failed to timeout member: {error}"))),
        }
    }
}
