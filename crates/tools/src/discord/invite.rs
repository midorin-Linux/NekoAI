use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{Value, json};
use serenity::{all::CreateInvite, http::Http};

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, get_bool, get_channel_id, get_guild_id_default, get_string, get_u8, get_u32, ok,
        to_value,
    },
};

pub struct GetDiscordInviteList {
    http: Arc<Http>,
}
pub struct CreateDiscordInvite {
    http: Arc<Http>,
}
pub struct DeleteDiscordInvite {
    http: Arc<Http>,
}

impl GetDiscordInviteList {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl CreateDiscordInvite {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl DeleteDiscordInvite {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for GetDiscordInviteList {
    const NAME: &'static str = "get_discord_invite_list";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List guild invites.".to_string(),
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
        match guild_id.invites(&self.http).await {
            Ok(invites) => Ok(ok(to_value(&invites))),
            Err(error) => Ok(err(format!("Failed to fetch invites: {error}"))),
        }
    }
}

impl Tool for CreateDiscordInvite {
    const NAME: &'static str = "create_discord_invite";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Create a channel invite.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id." },
                    "max_age": { "type": "integer", "description": "Max age in seconds." },
                    "max_uses": { "type": "integer", "description": "Max uses." },
                    "temporary": { "type": "boolean", "description": "Temporary membership." },
                    "unique": { "type": "boolean", "description": "Unique invite." }
                },
                "required": ["channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };

        let mut builder = CreateInvite::new();
        if let Some(max_age) = get_u32(&args, "max_age") {
            builder = builder.max_age(max_age);
        }
        if let Some(max_uses) = get_u8(&args, "max_uses") {
            builder = builder.max_uses(max_uses);
        }
        if let Some(temporary) = get_bool(&args, "temporary") {
            builder = builder.temporary(temporary);
        }
        if let Some(unique) = get_bool(&args, "unique") {
            builder = builder.unique(unique);
        }

        match channel_id.create_invite(&self.http, builder).await {
            Ok(invite) => Ok(ok(to_value(&invite))),
            Err(error) => Ok(err(format!("Failed to create invite: {error}"))),
        }
    }
}

impl Tool for DeleteDiscordInvite {
    const NAME: &'static str = "delete_discord_invite";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Delete an invite by code.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": { "code": { "type": "string", "description": "Invite code." } },
                "required": ["code"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(code) = get_string(&args, "code") else {
            return Ok(err("code is required"));
        };
        match self.http.delete_invite(code.as_str(), None).await {
            Ok(invite) => Ok(ok(to_value(&invite))),
            Err(error) => Ok(err(format!("Failed to delete invite: {error}"))),
        }
    }
}
