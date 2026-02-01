use crate::agent::tools::tools::build_tool;
use crate::agent::tools::discord::{
    err, get_bool, get_channel_id, get_guild_id_default, get_string, get_u16, get_u32, ok, parse_channel_type, to_value
};

use anyhow::Result;
use async_openai::types::chat::ChatCompletionTools;
use serde_json::{json, Value};
use serenity::all::{Context, CreateChannel, EditChannel};

pub fn definitions() -> Result<Vec<ChatCompletionTools>> {
    let mut tools = Vec::new();

    tools.push(build_tool(
        "create_channel",
        "Create a channel in a guild.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "name": { "type": "string", "description": "Channel name." },
                "kind": { "type": "string", "description": "Channel type (text, voice, category, news, stage, forum)." },
                "topic": { "type": "string", "description": "Channel topic." },
                "nsfw": { "type": "boolean", "description": "Whether the channel is NSFW." },
                "parent_id": { "type": "integer", "description": "Parent category channel id." },
                "position": { "type": "integer", "description": "Position in channel list." },
                "bitrate": { "type": "integer", "description": "Bitrate for voice channels." },
                "user_limit": { "type": "integer", "description": "User limit for voice channels." },
                "rate_limit_per_user": { "type": "integer", "description": "Slowmode in seconds." }
            },
            "required": ["guild_id", "name"]
        }),
    )?);

    tools.push(build_tool(
        "delete_channel",
        "Delete a channel.",
        json!({
            "type": "object",
            "properties": {
                "channel_id": { "type": "integer", "description": "Channel id." }
            },
            "required": ["channel_id"]
        }),
    )?);

    tools.push(build_tool(
        "modify_channel",
        "Modify channel settings.",
        json!({
            "type": "object",
            "properties": {
                "channel_id": { "type": "integer", "description": "Channel id." },
                "name": { "type": "string", "description": "New channel name." },
                "kind": { "type": "string", "description": "Channel type (text, voice, category, news, stage, forum)." },
                "topic": { "type": "string", "description": "New topic." },
                "nsfw": { "type": "boolean", "description": "Whether the channel is NSFW." },
                "parent_id": { "type": "integer", "description": "Parent category channel id." },
                "position": { "type": "integer", "description": "Position in channel list." },
                "bitrate": { "type": "integer", "description": "Bitrate for voice channels." },
                "user_limit": { "type": "integer", "description": "User limit for voice channels." },
                "rate_limit_per_user": { "type": "integer", "description": "Slowmode in seconds." }
            },
            "required": ["channel_id"]
        }),
    )?);

    tools.push(build_tool(
        "get_channel_info",
        "Get channel information.",
        json!({
            "type": "object",
            "properties": {
                "channel_id": { "type": "integer", "description": "Channel id." }
            },
            "required": ["channel_id"]
        }),
    )?);

    tools.push(build_tool(
        "get_channel_list",
        "Get channel list.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." }
            },
            "required": ["guild_id"]
        }),
    )?);

    Ok(tools)
}

async fn create_channel(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(name) = get_string(args, "name") else {
        return err("name is required");
    };

    let mut builder = CreateChannel::new(name);

    if let Some(kind) = args.get("kind").and_then(parse_channel_type) {
        builder = builder.kind(kind);
    }
    if let Some(topic) = get_string(args, "topic") {
        builder = builder.topic(topic);
    }
    if let Some(nsfw) = get_bool(args, "nsfw") {
        builder = builder.nsfw(nsfw);
    }
    if let Some(parent_id) = get_channel_id(args, "parent_id") {
        builder = builder.category(parent_id);
    }
    if let Some(position) = get_u16(args, "position") {
        builder = builder.position(position);
    }
    if let Some(bitrate) = get_u32(args, "bitrate") {
        builder = builder.bitrate(bitrate);
    }
    if let Some(user_limit) = get_u32(args, "user_limit") {
        builder = builder.user_limit(user_limit);
    }
    if let Some(rate_limit) = get_u16(args, "rate_limit_per_user") {
        builder = builder.rate_limit_per_user(rate_limit);
    }

    match guild_id.create_channel(ctx, builder).await {
        Ok(channel) => ok(to_value(&channel)),
        Err(error) => err(format!("Failed to create channel: {error}")),
    }
}

async fn delete_channel(ctx: &Context, args: &Value) -> String {
    let Some(channel_id) = get_channel_id(args, "channel_id") else {
        return err("channel_id is required");
    };

    match channel_id.delete(&ctx.http).await {
        Ok(channel) => ok(to_value(&channel)),
        Err(error) => err(format!("Failed to delete channel: {error}")),
    }
}

async fn modify_channel(ctx: &Context, args: &Value) -> String {
    let Some(channel_id) = get_channel_id(args, "channel_id") else {
        return err("channel_id is required");
    };

    let mut builder = EditChannel::new();
    let mut changed = false;

    if let Some(name) = get_string(args, "name") {
        builder = builder.name(name);
        changed = true;
    }
    if let Some(kind) = args.get("kind").and_then(parse_channel_type) {
        builder = builder.kind(kind);
        changed = true;
    }
    if let Some(topic) = get_string(args, "topic") {
        builder = builder.topic(topic);
        changed = true;
    }
    if let Some(nsfw) = get_bool(args, "nsfw") {
        builder = builder.nsfw(nsfw);
        changed = true;
    }
    if let Some(parent_id) = get_channel_id(args, "parent_id") {
        builder = builder.category(parent_id);
        changed = true;
    }
    if let Some(position) = get_u16(args, "position") {
        builder = builder.position(position);
        changed = true;
    }
    if let Some(bitrate) = get_u32(args, "bitrate") {
        builder = builder.bitrate(bitrate);
        changed = true;
    }
    if let Some(user_limit) = get_u32(args, "user_limit") {
        builder = builder.user_limit(user_limit);
        changed = true;
    }
    if let Some(rate_limit) = get_u16(args, "rate_limit_per_user") {
        builder = builder.rate_limit_per_user(rate_limit);
        changed = true;
    }

    if !changed {
        return err("No channel fields provided to modify");
    }

    match channel_id.edit(ctx, builder).await {
        Ok(channel) => ok(to_value(&channel)),
        Err(error) => err(format!("Failed to modify channel: {error}")),
    }
}

async fn get_channel_info(ctx: &Context, args: &Value) -> String {
    let Some(channel_id) = get_channel_id(args, "channel_id") else {
        return err("channel_id is required");
    };

    match channel_id.to_channel(ctx).await {
        Ok(channel) => ok(to_value(&channel)),
        Err(error) => err(format!("Failed to fetch channel info: {error}")),
    }
}

async fn get_channel_list(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };

    match guild_id.channels(&ctx.http).await {
        Ok(channels) => ok(to_value(&channels)),
        Err(error) => err(format!("Failed to fetch channel list: {error}")),
    }
}

pub async fn execute(ctx: &Context, name: &str, args: &Value) -> Option<String> {
    match name {
        "create_channel" => Some(create_channel(ctx, args).await),
        "delete_channel" => Some(delete_channel(ctx, args).await),
        "modify_channel" => Some(modify_channel(ctx, args).await),
        "get_channel_info" => Some(get_channel_info(ctx, args).await),
        "get_channel_list" => Some(get_channel_list(ctx, args).await),
        _ => None,
    }
}
