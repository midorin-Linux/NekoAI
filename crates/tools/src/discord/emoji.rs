use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{Value, json};
use serenity::{all::EmojiId, http::Http};

use crate::discord::{
    error::DiscordToolError,
    helpers::{err, get_guild_id_default, get_string, get_u64, ok, retry_discord, to_value},
};

pub struct GetDiscordEmojiList {
    http: Arc<Http>,
}
pub struct CreateDiscordEmoji {
    http: Arc<Http>,
}
pub struct DeleteDiscordEmoji {
    http: Arc<Http>,
}
pub struct GetDiscordStickerList {
    http: Arc<Http>,
}

impl GetDiscordEmojiList {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl CreateDiscordEmoji {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl DeleteDiscordEmoji {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl GetDiscordStickerList {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for GetDiscordEmojiList {
    const NAME: &'static str = "get_discord_emoji_list";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List custom emojis in a guild.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": { "guild_id": { "type": "integer", "description": "Guild id." } },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.emojis(&http).await }
        })
        .await
        {
            Ok(emojis) => Ok(ok(to_value(&emojis))),
            Err(error) => Ok(err(format!("Failed to fetch emojis: {error}"))),
        }
    }
}

impl Tool for CreateDiscordEmoji {
    const NAME: &'static str = "create_discord_emoji";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Create a custom emoji in a guild.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "name": { "type": "string", "description": "Emoji name." },
                    "image": { "type": "string", "description": "Base64 data URI for emoji image." }
                },
                "required": ["guild_id", "name", "image"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);
        let Some(name) = get_string(&args, "name") else {
            return Ok(err("name is required"));
        };
        let Some(image) = get_string(&args, "image") else {
            return Ok(err("image is required"));
        };

        let http = self.http.clone();
        let name = name.clone();
        let image = image.clone();
        match retry_discord(|| {
            let http = http.clone();
            let name = name.clone();
            let image = image.clone();
            async move {
                guild_id
                    .create_emoji(&http, name.as_str(), image.as_str())
                    .await
            }
        })
        .await
        {
            Ok(emoji) => Ok(ok(to_value(&emoji))),
            Err(error) => Ok(err(format!("Failed to create emoji: {error}"))),
        }
    }
}

impl Tool for DeleteDiscordEmoji {
    const NAME: &'static str = "delete_discord_emoji";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Delete a custom emoji from a guild.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "emoji_id": { "type": "integer", "description": "Emoji id." }
                },
                "required": ["guild_id", "emoji_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);
        let Some(emoji_id) = get_u64(&args, "emoji_id").map(EmojiId::new) else {
            return Ok(err("emoji_id is required"));
        };

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.delete_emoji(&http, emoji_id).await }
        })
        .await
        {
            Ok(()) => Ok(ok(json!({ "deleted": true }))),
            Err(error) => Ok(err(format!("Failed to delete emoji: {error}"))),
        }
    }
}

impl Tool for GetDiscordStickerList {
    const NAME: &'static str = "get_discord_sticker_list";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List guild stickers.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": { "guild_id": { "type": "integer", "description": "Guild id." } },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.stickers(&http).await }
        })
        .await
        {
            Ok(stickers) => Ok(ok(to_value(&stickers))),
            Err(error) => Ok(err(format!("Failed to fetch stickers: {error}"))),
        }
    }
}
