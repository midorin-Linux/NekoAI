use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{Value, json};
use serenity::{all::EmojiId, http::Http};

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, get_channel_id, get_guild_id_default, get_message_id, get_string, get_u64, ok,
        retry_discord, to_value,
    },
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

pub struct ListEmojis {
    inner: GetDiscordEmojiList,
}

pub struct AddEmoji {
    inner: CreateDiscordEmoji,
}

pub struct DeleteEmoji {
    inner: DeleteDiscordEmoji,
}

pub struct GetReactionStats {
    http: Arc<Http>,
}

impl ListEmojis {
    pub fn new(http: Arc<Http>) -> Self {
        Self {
            inner: GetDiscordEmojiList::new(http),
        }
    }
}

impl AddEmoji {
    pub fn new(http: Arc<Http>) -> Self {
        Self {
            inner: CreateDiscordEmoji::new(http),
        }
    }
}

impl DeleteEmoji {
    pub fn new(http: Arc<Http>) -> Self {
        Self {
            inner: DeleteDiscordEmoji::new(http),
        }
    }
}

impl GetReactionStats {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for ListEmojis {
    const NAME: &'static str = "list_emojis";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List custom guild emojis with metadata.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": { "guild_id": { "type": "integer" } },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.inner.call(args).await
    }
}

impl Tool for AddEmoji {
    const NAME: &'static str = "add_emoji";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Add a custom emoji from data URI/base64 image.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer" },
                    "name": { "type": "string" },
                    "image": { "type": "string", "description": "data:image/...;base64,..." }
                },
                "required": ["guild_id", "name", "image"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.inner.call(args).await
    }
}

impl Tool for DeleteEmoji {
    const NAME: &'static str = "delete_emoji";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Delete a custom guild emoji.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer" },
                    "emoji_id": { "type": "integer" }
                },
                "required": ["guild_id", "emoji_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.inner.call(args).await
    }
}

impl Tool for GetReactionStats {
    const NAME: &'static str = "get_reaction_stats";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get message reaction totals grouped by emoji.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer" },
                    "message_id": { "type": "integer" }
                },
                "required": ["channel_id", "message_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        let Some(message_id) = get_message_id(&args, "message_id") else {
            return Ok(err("message_id is required"));
        };

        let message = match retry_discord(|| {
            let http = self.http.clone();
            async move { channel_id.message(&http, message_id).await }
        })
        .await
        {
            Ok(message) => message,
            Err(error) => return Ok(err(format!("Failed to fetch message: {error}"))),
        };

        let reactions = message
            .reactions
            .iter()
            .map(|reaction| {
                json!({
                    "emoji": reaction.reaction_type.to_string(),
                    "count": reaction.count,
                    "me": reaction.me,
                })
            })
            .collect::<Vec<_>>();

        Ok(ok(json!({
            "channel_id": channel_id.get(),
            "message_id": message_id.get(),
            "reaction_count": reactions.len(),
            "reactions": reactions,
        })))
    }
}
