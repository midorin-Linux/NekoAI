use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{Value, json};
use serenity::{
    all::{AuditLogEntryId, CreateAttachment, EditGuild, GuildId, audit_log::Action},
    http::{GuildPagination, Http},
};

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, get_bool, get_guild_id_default, get_string, get_u8, get_u32, get_u64, get_user_id, ok,
        to_value,
    },
};

pub struct GetDiscordGuildInfo {
    http: Arc<Http>,
}

pub struct GetDiscordGuildList {
    http: Arc<Http>,
}

pub struct ModifyDiscordGuild {
    http: Arc<Http>,
}

pub struct GetDiscordAuditLog {
    http: Arc<Http>,
}

impl GetDiscordGuildInfo {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl GetDiscordGuildList {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl ModifyDiscordGuild {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl GetDiscordAuditLog {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for GetDiscordGuildInfo {
    const NAME: &'static str = "get_discord_guild_info";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get guild information.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." }
                },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };

        match guild_id.to_partial_guild(&self.http).await {
            Ok(guild) => Ok(ok(to_value(&guild))),
            Err(error) => Ok(err(format!("Failed to fetch guild info: {error}"))),
        }
    }
}

impl Tool for GetDiscordGuildList {
    const NAME: &'static str = "get_discord_guild_list";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List guilds bot is in.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max guilds to return (1-200)." },
                    "after": { "type": "integer", "description": "Return guilds after this guild id." }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let limit = get_u64(&args, "limit");
        let after = get_u64(&args, "after");

        let pagination = after.map(|guild_id| GuildPagination::After(GuildId::new(guild_id)));

        match self.http.get_guilds(pagination, limit).await {
            Ok(guilds) => Ok(ok(to_value(&guilds))),
            Err(error) => Ok(err(format!("Failed to fetch guild list: {error}"))),
        }
    }
}

impl Tool for ModifyDiscordGuild {
    const NAME: &'static str = "modify_discord_guild";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Modify guild settings such as name or icon.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "name": { "type": "string", "description": "New guild name." },
                    "description": { "type": "string", "description": "New guild description." },
                    "icon_path": { "type": "string", "description": "Local path to PNG icon file." },
                    "clear_icon": { "type": "boolean", "description": "Clear current icon." }
                },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let mut builder = EditGuild::new();
        let mut changed = false;

        if let Some(name) = get_string(&args, "name") {
            builder = builder.name(name);
            changed = true;
        }
        if let Some(description) = get_string(&args, "description") {
            builder = builder.description(description);
            changed = true;
        }
        if let Some(true) = get_bool(&args, "clear_icon") {
            builder = builder.icon(None);
            changed = true;
        } else if let Some(icon_path) = get_string(&args, "icon_path") {
            match std::fs::read(&icon_path) {
                Ok(icon_data) => {
                    let attachment = CreateAttachment::bytes(icon_data, "icon.png");
                    builder = builder.icon(Some(&attachment));
                    changed = true;
                }
                Err(error) => return Ok(err(format!("Failed to read icon file: {error}"))),
            }
        }

        if !changed {
            return Ok(err("No guild fields provided to modify"));
        }

        match guild_id.edit(&self.http, builder).await {
            Ok(guild) => Ok(ok(to_value(&guild))),
            Err(error) => Ok(err(format!("Failed to modify guild: {error}"))),
        }
    }
}

impl Tool for GetDiscordAuditLog {
    const NAME: &'static str = "get_discord_audit_log";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Fetch guild audit log entries.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "action_type": { "type": "integer", "description": "Audit log action type number." },
                    "user_id": { "type": "integer", "description": "Filter by user id." },
                    "before": { "type": "integer", "description": "Fetch entries before this audit log entry id." },
                    "limit": { "type": "integer", "description": "Number of entries to return." }
                },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };

        let limit = get_u8(&args, "limit");
        let action_type = get_u32(&args, "action_type").map(|v| Action::from_value(v as u8));
        let user_id = get_user_id(&args, "user_id");
        let before = get_u64(&args, "before").map(AuditLogEntryId::new);

        match guild_id
            .audit_logs(&self.http, action_type, user_id, before, limit)
            .await
        {
            Ok(log) => Ok(ok(to_value(&log))),
            Err(error) => Ok(err(format!("Failed to fetch audit log: {error}"))),
        }
    }
}
