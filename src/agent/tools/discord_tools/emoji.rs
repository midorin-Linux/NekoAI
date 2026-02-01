use crate::agent::tools::tools::build_tool;
use crate::agent::tools::discord::{
    err, get_guild_id_default, get_string, get_u64, ok, to_value
};

use anyhow::Result;
use async_openai::types::chat::ChatCompletionTools;
use serde_json::{json, Value};
use serenity::all::{Context, EmojiId};

pub fn definitions() -> Result<Vec<ChatCompletionTools>> {
    let mut tools = Vec::new();

    tools.push(build_tool(
        "get_emoji_list",
        "List custom emojis in a guild.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." }
            },
            "required": ["guild_id"]
        }),
    )?);

    tools.push(build_tool(
        "create_emoji",
        "Create a custom emoji in a guild.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "name": { "type": "string", "description": "Emoji name." },
                "image": { "type": "string", "description": "Base64 data URI for emoji image." }
            },
            "required": ["guild_id", "name", "image"]
        }),
    )?);

    tools.push(build_tool(
        "delete_emoji",
        "Delete a custom emoji from a guild.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "emoji_id": { "type": "integer", "description": "Emoji id." }
            },
            "required": ["guild_id", "emoji_id"]
        }),
    )?);

    tools.push(build_tool(
        "get_sticker_list",
        "List guild stickers.",
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

async fn get_emoji_list(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };

    match guild_id.emojis(&ctx.http).await {
        Ok(emojis) => ok(to_value(&emojis)),
        Err(error) => err(format!("Failed to fetch emojis: {error}")),
    }
}

async fn create_emoji(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(name) = get_string(args, "name") else {
        return err("name is required");
    };
    let Some(image) = get_string(args, "image") else {
        return err("image is required");
    };

    match guild_id.create_emoji(&ctx.http, name.as_str(), image.as_str()).await {
        Ok(emoji) => ok(to_value(&emoji)),
        Err(error) => err(format!("Failed to create emoji: {error}")),
    }
}

async fn delete_emoji(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(emoji_id) = get_u64(args, "emoji_id").map(EmojiId::new) else {
        return err("emoji_id is required");
    };

    match guild_id.delete_emoji(&ctx.http, emoji_id).await {
        Ok(()) => ok(json!({ "deleted": true })),
        Err(error) => err(format!("Failed to delete emoji: {error}")),
    }
}

async fn get_sticker_list(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };

    match guild_id.stickers(&ctx.http).await {
        Ok(stickers) => ok(to_value(&stickers)),
        Err(error) => err(format!("Failed to fetch stickers: {error}")),
    }
}

pub async fn execute(ctx: &Context, name: &str, args: &Value) -> Option<String> {
    match name {
        "get_emoji_list" => Some(get_emoji_list(ctx, args).await),
        "create_emoji" => Some(create_emoji(ctx, args).await),
        "delete_emoji" => Some(delete_emoji(ctx, args).await),
        "get_sticker_list" => Some(get_sticker_list(ctx, args).await),
        _ => None,
    }
}
