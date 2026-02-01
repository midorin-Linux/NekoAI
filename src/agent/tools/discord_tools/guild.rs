use crate::agent::tools::tools::build_tool;
use crate::agent::tools::discord::{
    err, get_bool, get_guild_id_default, get_string, get_u32, get_u64, get_u8, get_user_id, ok, to_value
};

use anyhow::Result;
use async_openai::types::chat::ChatCompletionTools;
use serde_json::{json, Value};
use serenity::all::{Context, EditGuild, GuildId};

pub fn definitions() -> Result<Vec<ChatCompletionTools>> {
    let mut tools = Vec::new();

    tools.push(build_tool(
        "get_guild_info",
        "Get guild information.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." }
            },
            "required": ["guild_id"]
        }),
    )?);

    tools.push(build_tool(
        "get_guild_list",
        "List guilds bot is in.",
        json!({
            "type": "object",
            "properties": {
                "limit": { "type": "integer", "description": "Max guilds to return (1-200)." },
                "after": { "type": "integer", "description": "Return guilds after this guild id." }
            }
        }),
    )?);

    tools.push(build_tool(
        "modify_guild",
        "Modify guild settings such as name or icon.",
        json!({
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
    )?);

    tools.push(build_tool(
        "get_audit_log",
        "Fetch guild audit log entries.",
        json!({
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
    )?);

    Ok(tools)
}

async fn get_guild_info(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };

    match guild_id.to_partial_guild(ctx).await {
        Ok(guild) => ok(to_value(&guild)),
        Err(error) => err(format!("Failed to fetch guild info: {error}")),
    }
}

async fn get_guild_list(ctx: &Context, args: &Value) -> String {
    let limit = get_u64(args, "limit");
    let after = get_u64(args, "after");

    let pagination = match after {
        Some(guild_id) => Some(serenity::http::GuildPagination::After(GuildId::new(guild_id))),
        None => None,
    };

    match ctx.http.get_guilds(pagination, limit).await {
        Ok(guilds) => ok(to_value(&guilds)),
        Err(error) => err(format!("Failed to fetch guild list: {error}")),
    }
}

async fn modify_guild(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };

    let mut builder = EditGuild::new();
    let mut changed = false;

    if let Some(name) = get_string(args, "name") {
        builder = builder.name(name);
        changed = true;
    }
    if let Some(description) = get_string(args, "description") {
        builder = builder.description(description);
        changed = true;
    }
    if let Some(true) = get_bool(args, "clear_icon") {
        builder = builder.icon(None);
        changed = true;
    } else if let Some(icon_path) = get_string(args, "icon_path") {
        match tokio::fs::read(&icon_path).await {
            Ok(icon_data) => {
                let attachment = serenity::all::CreateAttachment::bytes(icon_data, "icon.png");
                builder = builder.icon(Some(&attachment));
                changed = true;
            }
            Err(error) => return err(format!("Failed to read icon file: {error}")),
        }
    }

    if !changed {
        return err("No guild fields provided to modify");
    }

    match guild_id.edit(ctx, builder).await {
        Ok(guild) => ok(to_value(&guild)),
        Err(error) => err(format!("Failed to modify guild: {error}")),
    }
}

async fn get_audit_log(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };

    let limit = get_u8(args, "limit");
    let action_type = get_u32(args, "action_type")
        .map(|v| serenity::all::audit_log::Action::from_value(v as u8));
    let user_id = get_user_id(args, "user_id");
    let before = get_u64(args, "before")
        .map(|v| serenity::all::AuditLogEntryId::new(v));

    match guild_id.audit_logs(&ctx.http, action_type, user_id, before, limit).await {
        Ok(log) => ok(to_value(&log)),
        Err(error) => err(format!("Failed to fetch audit log: {error}")),
    }
}

pub async fn execute(ctx: &Context, name: &str, args: &Value) -> Option<String> {
    match name {
        "get_guild_info" => Some(get_guild_info(ctx, args).await),
        "get_guild_list" => Some(get_guild_list(ctx, args).await),
        "modify_guild" => Some(modify_guild(ctx, args).await),
        "get_audit_log" => Some(get_audit_log(ctx, args).await),
        _ => None,
    }
}
