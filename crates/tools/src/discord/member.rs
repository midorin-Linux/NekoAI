use std::sync::Arc;

use chrono::Utc;
use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{Value, json};
use serenity::{
    all::{EditMember, Permissions, RoleId, UserId},
    http::Http,
};

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, fetch_guild_members, get_bool, get_channel_id, get_guild_id_default, get_string,
        get_u8, get_u32, get_u64, get_u64_list, get_user_id, ok, parse_timestamp,
        resolve_relative_timestamp, resolve_role_id, resolve_role_ids, resolve_user_id,
        retry_discord, snowflake_to_datetime, to_value,
    },
    permission::require_current_user_admin,
};

pub struct GetDiscordMemberList {
    http: Arc<Http>,
}
pub struct GetDiscordMemberInfo {
    http: Arc<Http>,
}
pub struct KickDiscordMember {
    http: Arc<Http>,
}
pub struct BanDiscordMember {
    http: Arc<Http>,
}
pub struct UnbanDiscordMember {
    http: Arc<Http>,
}
pub struct BulkBanDiscordMembers {
    http: Arc<Http>,
}
pub struct ModifyDiscordMember {
    http: Arc<Http>,
}
pub struct TimeoutDiscordMember {
    http: Arc<Http>,
}

impl GetDiscordMemberList {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl GetDiscordMemberInfo {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl KickDiscordMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl BanDiscordMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl UnbanDiscordMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl BulkBanDiscordMembers {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl ModifyDiscordMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl TimeoutDiscordMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

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
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        let limit = get_u64(&args, "limit");
        let after = get_user_id(&args, "after");

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.members(&http, limit, after).await }
        })
        .await
        {
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
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        let Some(user_id) = get_user_id(&args, "user_id") else {
            return Ok(err("user_id is required"));
        };

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.member(&http, user_id).await }
        })
        .await
        {
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
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        if let Err(message) = require_current_user_admin(&self.http, guild_id).await {
            return Ok(err(message));
        }
        let Some(user_id) = get_user_id(&args, "user_id") else {
            return Ok(err("user_id is required"));
        };
        let reason = get_string(&args, "reason");

        let http = self.http.clone();
        let reason = reason.clone();
        match retry_discord(|| {
            let http = http.clone();
            let reason = reason.clone();
            async move {
                guild_id
                    .kick_with_reason(&http, user_id, reason.as_deref().unwrap_or(""))
                    .await
            }
        })
        .await
        {
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
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        if let Err(message) = require_current_user_admin(&self.http, guild_id).await {
            return Ok(err(message));
        }
        let Some(user_id) = get_user_id(&args, "user_id") else {
            return Ok(err("user_id is required"));
        };
        let delete_message_days = get_u8(&args, "delete_message_days").unwrap_or(0);
        let reason = get_string(&args, "reason");

        let http = self.http.clone();
        let reason = reason.clone();
        match retry_discord(|| {
            let http = http.clone();
            let reason = reason.clone();
            async move {
                guild_id
                    .ban_with_reason(
                        &http,
                        user_id,
                        delete_message_days,
                        reason.as_deref().unwrap_or(""),
                    )
                    .await
            }
        })
        .await
        {
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
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        if let Err(message) = require_current_user_admin(&self.http, guild_id).await {
            return Ok(err(message));
        }
        let Some(user_id) = get_user_id(&args, "user_id") else {
            return Ok(err("user_id is required"));
        };

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.unban(&http, user_id).await }
        })
        .await
        {
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
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        if let Err(message) = require_current_user_admin(&self.http, guild_id).await {
            return Ok(err(message));
        }
        let Some(user_ids) = get_u64_list(&args, "user_ids") else {
            return Ok(err("user_ids is required"));
        };
        let delete_message_seconds = get_u32(&args, "delete_message_seconds").unwrap_or(0);
        let reason = get_string(&args, "reason");
        let user_ids = user_ids.into_iter().map(UserId::new).collect::<Vec<_>>();

        let http = self.http.clone();
        let user_ids = user_ids.clone();
        let reason = reason.clone();
        match retry_discord(|| {
            let http = http.clone();
            let user_ids = user_ids.clone();
            let reason = reason.clone();
            async move {
                guild_id
                    .bulk_ban(
                        http.as_ref(),
                        &user_ids,
                        delete_message_seconds,
                        reason.as_deref(),
                    )
                    .await
            }
        })
        .await
        {
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
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        if let Err(message) = require_current_user_admin(&self.http, guild_id).await {
            return Ok(err(message));
        }
        let Some(user_id) = get_user_id(&args, "user_id") else {
            return Ok(err("user_id is required"));
        };

        let mut builder = EditMember::new();
        let mut changed = false;

        if let Some(nick) = get_string(&args, "nick") {
            builder = builder.nickname(nick);
            changed = true;
        }
        if let Some(roles) = get_u64_list(&args, "roles") {
            let role_ids: Vec<RoleId> = roles.into_iter().map(RoleId::new).collect();
            builder = builder.roles(role_ids);
            changed = true;
        }
        if let Some(mute) = get_bool(&args, "mute") {
            builder = builder.mute(mute);
            changed = true;
        }
        if let Some(deafen) = get_bool(&args, "deafen") {
            builder = builder.deafen(deafen);
            changed = true;
        }
        if let Some(channel_id) = get_channel_id(&args, "channel_id") {
            builder = builder.voice_channel(channel_id);
            changed = true;
        }
        if let Some(true) = get_bool(&args, "disconnect") {
            builder = builder.disconnect_member();
            changed = true;
        }
        if let Some(true) = get_bool(&args, "clear_timeout") {
            builder = builder.enable_communication();
            changed = true;
        } else if let Some(until) = args
            .get("communication_disabled_until")
            .and_then(parse_timestamp)
        {
            builder = builder.disable_communication_until_datetime(until);
            changed = true;
        }

        if !changed {
            return Ok(err("No member fields provided to modify"));
        }

        let http = self.http.clone();
        let builder = builder.clone();
        match retry_discord(|| {
            let http = http.clone();
            let builder = builder.clone();
            async move { guild_id.edit_member(&http, user_id, builder).await }
        })
        .await
        {
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
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        if let Err(message) = require_current_user_admin(&self.http, guild_id).await {
            return Ok(err(message));
        }
        let Some(user_id) = get_user_id(&args, "user_id") else {
            return Ok(err("user_id is required"));
        };

        let builder = if let Some(true) = get_bool(&args, "clear") {
            EditMember::new().enable_communication()
        } else if let Some(until) = args.get("until").and_then(parse_timestamp) {
            EditMember::new().disable_communication_until_datetime(until)
        } else {
            return Ok(err("Either 'until' or 'clear' is required"));
        };

        let http = self.http.clone();
        let builder = builder.clone();
        match retry_discord(|| {
            let http = http.clone();
            let builder = builder.clone();
            async move { guild_id.edit_member(&http, user_id, builder).await }
        })
        .await
        {
            Ok(member) => Ok(ok(to_value(&member))),
            Err(error) => Ok(err(format!("Failed to timeout member: {error}"))),
        }
    }
}

pub struct SearchMembers {
    http: Arc<Http>,
}

pub struct ManageMemberRoles {
    http: Arc<Http>,
}

pub struct TimeoutMember {
    http: Arc<Http>,
}

pub struct InvestigateMember {
    http: Arc<Http>,
}

pub struct ModerateMember {
    http: Arc<Http>,
}

impl SearchMembers {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl ManageMemberRoles {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl TimeoutMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl InvestigateMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl ModerateMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for SearchMembers {
    const NAME: &'static str = "search_members";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Search guild members by name, role, or timeout status. Returns matching members with their ID, name, nickname, and key info. Supports searching by partial name or full name.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild ID." },
                    "query": { "type": "string", "description": "User name, nickname, or display name to search for (partial match)." },
                    "role_names": { "type": "array", "items": { "type": "string" }, "description": "Filter by role names (users with ANY of these roles)." },
                    "has_timeout": { "type": "boolean", "description": "Filter: only members who are currently timed out." },
                    "limit": { "type": "integer", "description": "Maximum number of results (default 20, max 100)." }
                },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };

        let query = get_string(&args, "query");
        let role_names: Vec<String> = args
            .get("role_names")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        let has_timeout = get_bool(&args, "has_timeout");
        let limit = get_u64(&args, "limit").unwrap_or(20).min(100);

        let role_ids = if role_names.is_empty() {
            Vec::new()
        } else {
            let role_ids = resolve_role_ids(&self.http, guild_id, &role_names).await;
            if role_ids.is_empty() {
                return Ok(err(format!(
                    "Could not resolve any roles from role_names: {}",
                    role_names.join(", ")
                )));
            }
            role_ids
        };

        let http = self.http.clone();
        let members = match retry_discord(|| {
            let http = http.clone();
            async move { fetch_guild_members(&http, guild_id, 5_000).await }
        })
        .await
        {
            Ok(members) => members,
            Err(e) => return Ok(err(format!("Failed to fetch members: {e}"))),
        };

        let filtered: Vec<_> = members
            .into_iter()
            .filter(|m| {
                if let Some(q) = &query {
                    let q_lower = q.to_lowercase();
                    let name_match = m.user.name.to_lowercase().contains(&q_lower);
                    let nick_match = m
                        .nick
                        .as_ref()
                        .is_some_and(|n| n.to_lowercase().contains(&q_lower));
                    let global_match = m
                        .user
                        .global_name
                        .as_ref()
                        .is_some_and(|g| g.to_lowercase().contains(&q_lower));
                    if !name_match && !nick_match && !global_match {
                        return false;
                    }
                }

                if !role_ids.is_empty() && !m.roles.iter().any(|r| role_ids.contains(r)) {
                    return false;
                }

                if let Some(timeout) = has_timeout {
                    let is_timed_out = m.communication_disabled_until.is_some();
                    if is_timed_out != timeout {
                        return false;
                    }
                }

                true
            })
            .collect();

        let total_matches = filtered.len();
        let members = filtered
            .into_iter()
            .take(limit as usize)
            .map(|m| {
                json!({
                    "id": m.user.id.get(),
                    "name": m.user.name,
                    "global_name": m.user.global_name,
                    "nick": m.nick,
                    "is_pending": m.pending,
                    "has_timeout": m.communication_disabled_until.is_some(),
                    "joined_at": m.joined_at.map(|t| t.to_string()),
                    "role_count": m.roles.len(),
                })
            })
            .collect::<Vec<_>>();

        Ok(ok(
            json!({ "total_matches": total_matches, "members": members }),
        ))
    }
}

impl Tool for ManageMemberRoles {
    const NAME: &'static str = "manage_member_roles";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Add or remove a role from a guild member. Accepts user name, mention, or ID for both the member and the role.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild ID." },
                    "target": { "type": "string", "description": "User name, @mention, or user ID." },
                    "action": { "type": "string", "enum": ["add", "remove"], "description": "Whether to add or remove the role." },
                    "role_query": { "type": "string", "description": "Role name, @mention, or role ID." }
                },
                "required": ["guild_id", "target", "action", "role_query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let target = match get_string(&args, "target") {
            Some(t) => t,
            None => return Ok(err("target is required")),
        };
        let action = match get_string(&args, "action") {
            Some(a) => a,
            None => return Ok(err("action is required (add or remove)")),
        };
        let role_query = match get_string(&args, "role_query") {
            Some(r) => r,
            None => return Ok(err("role_query is required")),
        };

        let user_id = match resolve_user_id(&self.http, guild_id, &target).await {
            Some(id) => id,
            None => return Ok(err(format!("Could not resolve user: {target}"))),
        };

        let role_id = match resolve_role_id(&self.http, guild_id, &role_query).await {
            Some(id) => id,
            None => return Ok(err(format!("Could not resolve role: {role_query}"))),
        };

        let http = self.http.clone();
        let member = match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.member(&http, user_id).await }
        })
        .await
        {
            Ok(member) => member,
            Err(e) => return Ok(err(format!("Failed to fetch member: {e}"))),
        };

        match action.as_str() {
            "add" => {
                if member.roles.contains(&role_id) {
                    return Ok(ok(json!({
                        "action": "add",
                        "already_had_role": true,
                        "user_id": user_id.get(),
                        "role_id": role_id.get()
                    })));
                }

                let http = self.http.clone();
                let member_clone = member.clone();
                retry_discord(|| {
                    let http = http.clone();
                    let member_clone = member_clone.clone();
                    async move { member_clone.add_role(&http, role_id).await }
                })
                .await?;

                Ok(ok(json!({
                    "action": "add",
                    "success": true,
                    "user_id": user_id.get(),
                    "role_id": role_id.get()
                })))
            }
            "remove" => {
                if !member.roles.contains(&role_id) {
                    return Ok(ok(json!({
                        "action": "remove",
                        "did_not_have_role": true,
                        "user_id": user_id.get(),
                        "role_id": role_id.get()
                    })));
                }

                let http = self.http.clone();
                let member_clone = member.clone();
                retry_discord(|| {
                    let http = http.clone();
                    let member_clone = member_clone.clone();
                    async move { member_clone.remove_role(&http, role_id).await }
                })
                .await?;

                Ok(ok(json!({
                    "action": "remove",
                    "success": true,
                    "user_id": user_id.get(),
                    "role_id": role_id.get()
                })))
            }
            other => Ok(err(format!(
                "Invalid action '{other}'. Use 'add' or 'remove'."
            ))),
        }
    }
}

impl Tool for TimeoutMember {
    const NAME: &'static str = "timeout_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Timeout a member using a relative duration, or clear an existing timeout. Examples: \"10m\", \"1h\", \"1d\", \"clear\".".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild ID." },
                    "target": { "type": "string", "description": "User name, @mention, or user ID." },
                    "duration": { "type": "string", "description": "Duration string like \"10m\", \"1h\", \"1d\", or \"clear\" to remove timeout." },
                    "reason": { "type": "string", "description": "Audit log reason." }
                },
                "required": ["guild_id", "target", "duration"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let target = match get_string(&args, "target") {
            Some(t) => t,
            None => return Ok(err("target is required")),
        };
        let duration = match get_string(&args, "duration") {
            Some(d) => d,
            None => return Ok(err("duration is required")),
        };
        let reason = get_string(&args, "reason");

        let user_id = match resolve_user_id(&self.http, guild_id, &target).await {
            Some(id) => id,
            None => return Ok(err(format!("Could not resolve user: {target}"))),
        };

        let builder = if duration == "clear" {
            EditMember::new().enable_communication()
        } else if let Some(timestamp) = resolve_relative_timestamp(&duration) {
            EditMember::new().disable_communication_until_datetime(timestamp)
        } else {
            return Ok(err(format!(
                "Invalid duration '{duration}'. Use a relative time like '10m', '1h', '1d', or 'clear'."
            )));
        };

        let http = self.http.clone();
        let builder = builder.clone();
        match retry_discord(|| {
            let http = http.clone();
            let builder = builder.clone();
            async move { guild_id.edit_member(&http, user_id, builder).await }
        })
        .await
        {
            Ok(member) => Ok(ok(json!({
                "timeout_active": member.communication_disabled_until.is_some(),
                "communication_disabled_until": member.communication_disabled_until.map(|t| t.to_string()),
                "reason": reason,
            }))),
            Err(e) => Ok(err(format!("Failed to timeout member: {e}"))),
        }
    }
}

impl Tool for InvestigateMember {
    const NAME: &'static str = "investigate_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get a comprehensive profile of a guild member including account age, join date, roles, permissions, timeout status, and more.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild ID." },
                    "target": { "type": "string", "description": "User name, @mention, or user ID." }
                },
                "required": ["guild_id", "target"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        let target = match get_string(&args, "target") {
            Some(t) => t,
            None => return Ok(err("target is required")),
        };

        let user_id = match resolve_user_id(&self.http, guild_id, &target).await {
            Some(id) => id,
            None => return Ok(err(format!("Could not resolve user: {target}"))),
        };

        let http = self.http.clone();
        let guild = match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.to_partial_guild(&http).await }
        })
        .await
        {
            Ok(guild) => guild,
            Err(e) => return Ok(err(format!("Failed to fetch guild info: {e}"))),
        };

        let http = self.http.clone();
        let member = match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.member(&http, user_id).await }
        })
        .await
        {
            Ok(member) => member,
            Err(e) => return Ok(err(format!("Failed to fetch member: {e}"))),
        };

        let http = self.http.clone();
        let roles_map = retry_discord(|| {
            let http = http.clone();
            async move { guild_id.roles(&http).await }
        })
        .await
        .unwrap_or_else(|_| Default::default());

        let member_roles: Vec<Value> = member
            .roles
            .iter()
            .filter_map(|role_id| roles_map.get(role_id))
            .map(|role| {
                json!({
                    "id": role.id.get(),
                    "name": role.name,
                    "color": role.colour.hex(),
                    "position": role.position,
                })
            })
            .collect();

        let account_created = snowflake_to_datetime(member.user.id.get());
        let account_age_days = (Utc::now() - account_created).num_days();
        let is_new_account = account_age_days < 7;
        let is_owner = guild.owner_id == member.user.id;
        let is_administrator = guild
            .member_permissions(&member)
            .contains(Permissions::ADMINISTRATOR);

        Ok(ok(json!({
            "user": {
                "id": member.user.id.get(),
                "name": member.user.name,
                "global_name": member.user.global_name,
                "discriminator": member.user.discriminator,
                "bot": member.user.bot,
            },
            "account": {
                "created_at": account_created.to_rfc3339(),
                "age_days": account_age_days,
                "is_new_account": is_new_account,
            },
            "membership": {
                "nick": member.nick,
                "joined_at": member.joined_at.map(|t| t.to_string()),
                "is_pending": member.pending,
            },
            "roles": {
                "count": member.roles.len(),
                "list": member_roles,
            },
            "timeout": {
                "is_timed_out": member.communication_disabled_until.is_some(),
                "disabled_until": member.communication_disabled_until.map(|t| t.to_string()),
            },
            "permissions": {
                "is_owner": is_owner,
                "is_administrator": is_administrator,
            },
            "other": {
                "premium_since": member.premium_since.map(|t| t.to_string()),
                "avatar": member.avatar.map(|a| a.to_string()),
            },
        })))
    }
}

impl Tool for ModerateMember {
    const NAME: &'static str = "moderate_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Kick, ban, or softban (ban + immediate unban) a member from the guild. Supports relative message deletion periods.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild ID." },
                    "target": { "type": "string", "description": "User name, @mention, or user ID." },
                    "action": { "type": "string", "enum": ["kick", "ban", "softban"], "description": "Action to take. 'softban' bans then immediately unbans (deletes messages)." },
                    "delete_messages": { "type": "string", "enum": ["none", "1d", "7d"], "description": "Delete message history (only applies to ban/softban). Default: 'none'.", "default": "none" },
                    "reason": { "type": "string", "description": "Audit log reason." }
                },
                "required": ["guild_id", "target", "action"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let target = match get_string(&args, "target") {
            Some(t) => t,
            None => return Ok(err("target is required")),
        };
        let action = match get_string(&args, "action") {
            Some(a) => a,
            None => return Ok(err("action is required (kick, ban, or softban)")),
        };
        let delete_messages =
            get_string(&args, "delete_messages").unwrap_or_else(|| "none".to_string());
        let reason = get_string(&args, "reason").unwrap_or_default();

        let user_id = match resolve_user_id(&self.http, guild_id, &target).await {
            Some(id) => id,
            None => return Ok(err(format!("Could not resolve user: {target}"))),
        };

        let delete_days: u8 = match delete_messages.as_str() {
            "1d" => 1,
            "7d" => 7,
            _ => 0,
        };

        match action.as_str() {
            "kick" => {
                let http = self.http.clone();
                retry_discord(|| {
                    let http = http.clone();
                    let reason = reason.clone();
                    async move {
                        guild_id
                            .kick_with_reason(&http, user_id, reason.as_str())
                            .await
                    }
                })
                .await?;

                Ok(ok(json!({
                    "action": "kick",
                    "success": true,
                    "user_id": user_id.get(),
                    "reason": reason,
                })))
            }
            "ban" | "softban" => {
                let http = self.http.clone();
                retry_discord(|| {
                    let http = http.clone();
                    let reason = reason.clone();
                    async move {
                        guild_id
                            .ban_with_reason(&http, user_id, delete_days, reason.as_str())
                            .await
                    }
                })
                .await?;

                if action == "softban" {
                    let http = self.http.clone();
                    retry_discord(|| {
                        let http = http.clone();
                        async move { guild_id.unban(&http, user_id).await }
                    })
                    .await?;
                }

                Ok(ok(json!({
                    "action": action,
                    "success": true,
                    "user_id": user_id.get(),
                    "delete_message_days": delete_days,
                    "reason": reason,
                })))
            }
            other => Ok(err(format!(
                "Invalid action '{other}'. Use 'kick', 'ban', or 'softban'."
            ))),
        }
    }
}

pub struct UpdateMemberNickname {
    http: Arc<Http>,
}

pub struct KickMember {
    inner: KickDiscordMember,
}

pub struct GetMemberActivity {
    http: Arc<Http>,
}

impl UpdateMemberNickname {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl KickMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self {
            inner: KickDiscordMember::new(http),
        }
    }
}

impl GetMemberActivity {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for UpdateMemberNickname {
    const NAME: &'static str = "update_member_nickname";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Update a member's server nickname.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer" },
                    "user_id": { "type": "integer" },
                    "nickname": { "type": "string" }
                },
                "required": ["guild_id", "user_id", "nickname"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);
        let Some(user_id) = get_user_id(&args, "user_id") else {
            return Ok(err("user_id is required"));
        };
        let Some(nickname) = get_string(&args, "nickname") else {
            return Ok(err("nickname is required"));
        };

        let mut edit = EditMember::new();
        edit = edit.nickname(nickname);

        match retry_discord(|| {
            let http = self.http.clone();
            let edit = edit.clone();
            async move { guild_id.edit_member(&http, user_id, edit).await }
        })
        .await
        {
            Ok(updated) => Ok(ok(to_value(&updated))),
            Err(error) => Ok(err(format!("Failed to update nickname: {error}"))),
        }
    }
}

impl Tool for KickMember {
    const NAME: &'static str = "kick_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Kick a member from the guild with an optional reason.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer" },
                    "user_id": { "type": "integer" },
                    "reason": { "type": "string" }
                },
                "required": ["guild_id", "user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.inner.call(args).await
    }
}

impl Tool for GetMemberActivity {
    const NAME: &'static str = "get_member_activity";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description:
                "Get lightweight member activity signals such as join date and timeout state."
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer" },
                    "user_id": { "type": "integer" }
                },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };

        if let Some(user_id) = get_user_id(&args, "user_id") {
            let member = match retry_discord(|| {
                let http = self.http.clone();
                async move { guild_id.member(&http, user_id).await }
            })
            .await
            {
                Ok(member) => member,
                Err(error) => return Ok(err(format!("Failed to fetch member: {error}"))),
            };

            return Ok(ok(json!({
                "member": {
                    "user_id": member.user.id.get(),
                    "name": member.user.name,
                    "global_name": member.user.global_name,
                    "nick": member.nick,
                    "joined_at": member.joined_at.map(|value| value.to_string()),
                    "timed_out": member.communication_disabled_until.is_some(),
                    "boosting_since": member.premium_since.map(|value| value.to_string()),
                }
            })));
        }

        let members = match retry_discord(|| {
            let http = self.http.clone();
            async move { fetch_guild_members(&http, guild_id, 2_000).await }
        })
        .await
        {
            Ok(members) => members,
            Err(error) => return Ok(err(format!("Failed to fetch members: {error}"))),
        };

        let timed_out = members
            .iter()
            .filter(|member| member.communication_disabled_until.is_some())
            .count();
        let boosting = members
            .iter()
            .filter(|member| member.premium_since.is_some())
            .count();

        Ok(ok(json!({
            "summary": {
                "members": members.len(),
                "timed_out_members": timed_out,
                "boosting_members": boosting,
            }
        })))
    }
}
