use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::all::{ChannelId, Colour, CreateEmbed, CreateMessage, Http};

use super::error::DiscordToolError;

#[derive(Deserialize)]
pub struct EmbedField {
    name: String,
    value: String,
    #[serde(default)]
    inline: bool,
}

#[derive(Deserialize)]
pub struct SendEmbedArgs {
    channel_id: u64,
    title: Option<String>,
    description: Option<String>,
    color: Option<u32>,
    fields: Option<Vec<EmbedField>>,
    footer: Option<String>,
    thumbnail_url: Option<String>,
    image_url: Option<String>,
    author_name: Option<String>,
    url: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SendEmbed {
    #[serde(skip)]
    http: Option<Arc<Http>>,
}

impl SendEmbed {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http: Some(http) }
    }
}

impl Tool for SendEmbed {
    const NAME: &'static str = "send_embed";
    type Error = DiscordToolError;
    type Args = SendEmbedArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "send_embed".to_string(),
            description: "Send a rich embed message to a Discord channel with title, description, color, fields, footer, images, etc.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The Discord channel ID to send the embed to"
                    },
                    "title": {
                        "type": "string",
                        "description": "The embed title (optional)"
                    },
                    "description": {
                        "type": "string",
                        "description": "The embed description/body text (optional)"
                    },
                    "color": {
                        "type": "integer",
                        "description": "The embed color as an integer (e.g. 0xFF0000 for red) (optional)"
                    },
                    "fields": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string", "description": "Field name" },
                                "value": { "type": "string", "description": "Field value" },
                                "inline": { "type": "boolean", "description": "Display inline (default: false)" }
                            },
                            "required": ["name", "value"]
                        },
                        "description": "Embed fields (optional)"
                    },
                    "footer": {
                        "type": "string",
                        "description": "Footer text (optional)"
                    },
                    "thumbnail_url": {
                        "type": "string",
                        "description": "Thumbnail image URL (optional)"
                    },
                    "image_url": {
                        "type": "string",
                        "description": "Large image URL (optional)"
                    },
                    "author_name": {
                        "type": "string",
                        "description": "Author name displayed at the top (optional)"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL for the embed title link (optional)"
                    }
                },
                "required": ["channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("send_embed", "HTTP client not available"))?;

        let channel_id = ChannelId::new(args.channel_id);

        let mut embed = CreateEmbed::new();

        if let Some(title) = &args.title {
            embed = embed.title(title);
        }
        if let Some(description) = &args.description {
            embed = embed.description(description);
        }
        if let Some(color) = args.color {
            embed = embed.colour(Colour::new(color));
        }
        if let Some(fields) = &args.fields {
            for field in fields {
                embed = embed.field(&field.name, &field.value, field.inline);
            }
        }
        if let Some(footer) = &args.footer {
            embed = embed.footer(serenity::all::CreateEmbedFooter::new(footer));
        }
        if let Some(thumbnail_url) = &args.thumbnail_url {
            embed = embed.thumbnail(thumbnail_url);
        }
        if let Some(image_url) = &args.image_url {
            embed = embed.image(image_url);
        }
        if let Some(author_name) = &args.author_name {
            embed = embed.author(serenity::all::CreateEmbedAuthor::new(author_name));
        }
        if let Some(url) = &args.url {
            embed = embed.url(url);
        }

        let message = CreateMessage::new().embed(embed);

        channel_id
            .send_message(http.as_ref(), message)
            .await
            .map_err(|e| DiscordToolError::api_error("send_embed", e.to_string()))?;

        Ok(format!(
            "Successfully sent embed to channel {}",
            args.channel_id
        ))
    }
}
