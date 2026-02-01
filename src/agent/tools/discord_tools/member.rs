use crate::agent::tools::tools::build_tool;
use crate::agent::tools::discord::{
    err, get_bool, get_channel_id, get_guild_id_default, get_string, get_u32, get_u64, get_u64_list, get_u8, get_user_id, ok, parse_timestamp, to_value
};

use anyhow::Result;
use async_openai::types::chat::ChatCompletionTools;
use serde_json::{json, Value};
use serenity::all::{Context, EditMember, RoleId, UserId};

pub fn definitions() -> Result<Vec<ChatCompletionTools>> {
    let mut tools = Vec::new();

    tools.push(build_tool(
        "get_member_list",
        "List guild members.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "limit": { "type": "integer", "description": "Max members to return." },
                "after": { "type": "integer", "description": "Return members after this user id." }
            },
            "required": ["guild_id"]
        }),
    )?);

    tools.push(build_tool(
        "get_member_info",
        "Get guild member information.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "user_id": { "type": "integer", "description": "User id." }
            },
            "required": ["guild_id", "user_id"]
        }),
    )?);

    tools.push(build_tool(
        "kick_member",
        "Kick a member from the guild.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "user_id": { "type": "integer", "description": "User id." },
                "reason": { "type": "string", "description": "Audit log reason." }
            },
            "required": ["guild_id", "user_id"]
        }),
    )?);

    tools.push(build_tool(
        "ban_member",
        "Ban a member from the guild.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "user_id": { "type": "integer", "description": "User id." },
                "delete_message_days": { "type": "integer", "description": "Delete message history in days (0-7)." },
                "reason": { "type": "string", "description": "Audit log reason." }
            },
            "required": ["guild_id", "user_id"]
        }),
    )?);

    tools.push(build_tool(
        "unban_member",
        "Unban a user from the guild.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "user_id": { "type": "integer", "description": "User id." }
            },
            "required": ["guild_id", "user_id"]
        }),
    )?);

    tools.push(build_tool(
        "bulk_ban_members",
        "Bulk ban members from the guild.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "user_ids": { "type": "array", "items": { "type": "integer" }, "description": "User ids to ban." },
                "delete_message_seconds": { "type": "integer", "description": "Delete messages younger than this many seconds." },
                "reason": { "type": "string", "description": "Audit log reason." }
            },
            "required": ["guild_id", "user_ids"]
        }),
    )?);

    tools.push(build_tool(
        "modify_member",
        "Modify guild member settings.",
        json!({
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
    )?);

    tools.push(build_tool(
        "timeout_member",
        "Timeout or clear timeout for a member.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "user_id": { "type": "integer", "description": "User id." },
                "until": { "type": "string", "description": "RFC3339 timestamp to timeout until." },
                "clear": { "type": "boolean", "description": "Clear timeout." }
            },
            "required": ["guild_id", "user_id"]
        }),
    )?);

    Ok(tools)
}

async fn get_member_list(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let limit = get_u64(args, "limit");
    let after = get_user_id(args, "after");

    match guild_id.members(&ctx.http, limit, after).await {
        Ok(members) => ok(to_value(&members)),
        Err(error) => err(format!("Failed to fetch member list: {error}")),
    }
}

async fn get_member_info(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(user_id) = get_user_id(args, "user_id") else {
        return err("user_id is required");
    };

    match guild_id.member(&ctx.http, user_id).await {
        Ok(member) => ok(to_value(&member)),
        Err(error) => err(format!("Failed to fetch member info: {error}")),
    }
}

async fn kick_member(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(user_id) = get_user_id(args, "user_id") else {
        return err("user_id is required");
    };
    let reason = get_string(args, "reason");

    match guild_id.kick_with_reason(&ctx.http, user_id, reason.as_deref().unwrap_or("")).await {
        Ok(()) => ok(json!({ "kicked": true })),
        Err(error) => err(format!("Failed to kick member: {error}")),
    }
}

async fn ban_member(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(user_id) = get_user_id(args, "user_id") else {
        return err("user_id is required");
    };
    let delete_message_days = get_u8(args, "delete_message_days").unwrap_or(0);
    let reason = get_string(args, "reason");

    match guild_id
        .ban_with_reason(&ctx.http, user_id, delete_message_days, reason.as_deref().unwrap_or(""))
        .await
    {
        Ok(()) => ok(json!({ "banned": true })),
        Err(error) => err(format!("Failed to ban member: {error}")),
    }
}

async fn unban_member(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(user_id) = get_user_id(args, "user_id") else {
        return err("user_id is required");
    };

    match guild_id.unban(&ctx.http, user_id).await {
        Ok(()) => ok(json!({ "unbanned": true })),
        Err(error) => err(format!("Failed to unban member: {error}")),
    }
}

async fn bulk_ban_members(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(user_ids) = get_u64_list(args, "user_ids") else {
        return err("user_ids is required");
    };
    let delete_message_seconds = get_u32(args, "delete_message_seconds").unwrap_or(0);
    let reason = get_string(args, "reason");

    let user_ids: Vec<UserId> = user_ids.into_iter().map(UserId::new).collect();

    match guild_id
        .bulk_ban(&ctx.http, &user_ids, delete_message_seconds, reason.as_deref())
        .await
    {
        Ok(result) => ok(to_value(&result)),
        Err(error) => err(format!("Failed to bulk ban members: {error}")),
    }
}

async fn modify_member(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(user_id) = get_user_id(args, "user_id") else {
        return err("user_id is required");
    };

    let mut builder = EditMember::new();
    let mut changed = false;

    if let Some(nick) = get_string(args, "nick") {
        builder = builder.nickname(nick);
        changed = true;
    }
    if let Some(roles) = get_u64_list(args, "roles") {
        let role_ids: Vec<RoleId> = roles.into_iter().map(RoleId::new).collect();
        builder = builder.roles(&role_ids);
        changed = true;
    }
    if let Some(mute) = get_bool(args, "mute") {
        builder = builder.mute(mute);
        changed = true;
    }
    if let Some(deafen) = get_bool(args, "deafen") {
        builder = builder.deafen(deafen);
        changed = true;
    }
    if let Some(channel_id) = get_channel_id(args, "channel_id") {
        builder = builder.voice_channel(channel_id);
        changed = true;
    }
    if let Some(true) = get_bool(args, "disconnect") {
        builder = builder.disconnect_member();
        changed = true;
    }
    if let Some(true) = get_bool(args, "clear_timeout") {
        builder = builder.disable_communication_until(serenity::all::Timestamp::now().to_string());
        changed = true;
    } else if let Some(until) = args.get("communication_disabled_until").and_then(parse_timestamp) {
        builder = builder.disable_communication_until(until.to_string());
        changed = true;
    }

    if !changed {
        return err("No member fields provided to modify");
    }

    match guild_id.edit_member(ctx, user_id, builder).await {
        Ok(member) => ok(to_value(&member)),
        Err(error) => err(format!("Failed to modify member: {error}")),
    }
}

async fn timeout_member(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(user_id) = get_user_id(args, "user_id") else {
        return err("user_id is required");
    };

    let builder = if let Some(true) = get_bool(args, "clear") {
        EditMember::new().disable_communication_until(serenity::all::Timestamp::now().to_string())
    } else if let Some(until) = args.get("until").and_then(parse_timestamp) {
        EditMember::new().disable_communication_until(until.to_string())
    } else {
        return err("Either 'until' or 'clear' is required");
    };

    match guild_id.edit_member(ctx, user_id, builder).await {
        Ok(member) => ok(to_value(&member)),
        Err(error) => err(format!("Failed to timeout member: {error}")),
    }
}

pub async fn execute(ctx: &Context, name: &str, args: &Value) -> Option<String> {
    match name {
        "get_member_list" => Some(get_member_list(ctx, args).await),
        "get_member_info" => Some(get_member_info(ctx, args).await),
        "kick_member" => Some(kick_member(ctx, args).await),
        "ban_member" => Some(ban_member(ctx, args).await),
        "unban_member" => Some(unban_member(ctx, args).await),
        "bulk_ban_members" => Some(bulk_ban_members(ctx, args).await),
        "modify_member" => Some(modify_member(ctx, args).await),
        "timeout_member" => Some(timeout_member(ctx, args).await),
        _ => None,
    }
}
