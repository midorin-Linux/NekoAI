use std::sync::Arc;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde_json::{json, Value};
use serenity::all::{CreateScheduledEvent, EditScheduledEvent, ScheduledEventId};
use serenity::http::Http;

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, get_bool, get_channel_id, get_guild_id_default, get_string, get_u64, ok,
        parse_scheduled_event_status, parse_scheduled_event_type, parse_timestamp, to_value,
    },
};

pub struct GetDiscordScheduledEvents { http: Arc<Http> }
pub struct CreateDiscordScheduledEvent { http: Arc<Http> }
pub struct ModifyDiscordScheduledEvent { http: Arc<Http> }
pub struct DeleteDiscordScheduledEvent { http: Arc<Http> }

impl GetDiscordScheduledEvents { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl CreateDiscordScheduledEvent { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl ModifyDiscordScheduledEvent { pub fn new(http: Arc<Http>) -> Self { Self { http } } }
impl DeleteDiscordScheduledEvent { pub fn new(http: Arc<Http>) -> Self { Self { http } }
}

impl Tool for GetDiscordScheduledEvents {
    const NAME: &'static str = "get_discord_scheduled_events";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List scheduled events in a guild.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "with_user_count": { "type": "boolean", "description": "Include user counts." }
                },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let with_user_count = get_bool(&args, "with_user_count").unwrap_or(false);

        match guild_id.scheduled_events(&self.http, with_user_count).await {
            Ok(events) => Ok(ok(to_value(&events))),
            Err(error) => Ok(err(format!("Failed to fetch scheduled events: {error}"))),
        }
    }
}

impl Tool for CreateDiscordScheduledEvent {
    const NAME: &'static str = "create_discord_scheduled_event";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Create a scheduled event in a guild.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "name": { "type": "string", "description": "Event name." },
                    "start_time": { "type": "string", "description": "RFC3339 start timestamp." },
                    "kind": { "type": "string", "description": "Event type (voice, stage, external)." },
                    "channel_id": { "type": "integer", "description": "Channel id for voice/stage events." },
                    "end_time": { "type": "string", "description": "RFC3339 end timestamp." },
                    "description": { "type": "string", "description": "Event description." },
                    "location": { "type": "string", "description": "Location for external events." }
                },
                "required": ["guild_id", "name", "start_time", "kind"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let Some(name) = get_string(&args, "name") else { return Ok(err("name is required")); };
        let Some(start_time) = args.get("start_time").and_then(parse_timestamp) else { return Ok(err("start_time is required (RFC3339)")); };
        let Some(kind) = args.get("kind").and_then(parse_scheduled_event_type) else { return Ok(err("kind is required")); };

        let mut builder = CreateScheduledEvent::new(kind, name, start_time);
        if let Some(description) = get_string(&args, "description") { builder = builder.description(description); }
        if let Some(end_time) = args.get("end_time").and_then(parse_timestamp) { builder = builder.end_time(end_time); }
        if let Some(channel_id) = get_channel_id(&args, "channel_id") { builder = builder.channel_id(channel_id); }
        if let Some(location) = get_string(&args, "location") { builder = builder.location(location); }

        match guild_id.create_scheduled_event(&self.http, builder).await {
            Ok(event) => Ok(ok(to_value(&event))),
            Err(error) => Ok(err(format!("Failed to create scheduled event: {error}"))),
        }
    }
}

impl Tool for ModifyDiscordScheduledEvent {
    const NAME: &'static str = "modify_discord_scheduled_event";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Modify a scheduled event.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "event_id": { "type": "integer", "description": "Scheduled event id." },
                    "name": { "type": "string", "description": "Event name." },
                    "description": { "type": "string", "description": "Event description." },
                    "start_time": { "type": "string", "description": "RFC3339 start timestamp." },
                    "end_time": { "type": "string", "description": "RFC3339 end timestamp." },
                    "channel_id": { "type": "integer", "description": "Channel id for voice/stage events." },
                    "location": { "type": "string", "description": "Location for external events." },
                    "kind": { "type": "string", "description": "Event type (voice, stage, external)." },
                    "status": { "type": "string", "description": "Event status (scheduled, active, completed, canceled)." }
                },
                "required": ["guild_id", "event_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let Some(event_id) = get_u64(&args, "event_id").map(ScheduledEventId::new) else { return Ok(err("event_id is required")); };

        let mut builder = EditScheduledEvent::new();
        let mut changed = false;

        if let Some(name) = get_string(&args, "name") { builder = builder.name(name); changed = true; }
        if let Some(description) = get_string(&args, "description") { builder = builder.description(description); changed = true; }
        if let Some(start_time) = args.get("start_time").and_then(parse_timestamp) { builder = builder.start_time(start_time); changed = true; }
        if let Some(end_time) = args.get("end_time").and_then(parse_timestamp) { builder = builder.end_time(end_time); changed = true; }
        if let Some(channel_id) = get_channel_id(&args, "channel_id") { builder = builder.channel_id(channel_id); changed = true; }
        if let Some(location) = get_string(&args, "location") { builder = builder.location(location); changed = true; }
        if let Some(kind) = args.get("kind").and_then(parse_scheduled_event_type) { builder = builder.kind(kind); changed = true; }
        if let Some(status) = args.get("status").and_then(parse_scheduled_event_status) { builder = builder.status(status); changed = true; }

        if !changed { return Ok(err("No scheduled event fields provided to modify")); }

        match guild_id.edit_scheduled_event(&self.http, event_id, builder).await {
            Ok(event) => Ok(ok(to_value(&event))),
            Err(error) => Ok(err(format!("Failed to modify scheduled event: {error}"))),
        }
    }
}

impl Tool for DeleteDiscordScheduledEvent {
    const NAME: &'static str = "delete_discord_scheduled_event";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Delete a scheduled event.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "event_id": { "type": "integer", "description": "Scheduled event id." }
                },
                "required": ["guild_id", "event_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else { return Ok(err("guild_id is required")); };
        let Some(event_id) = get_u64(&args, "event_id").map(ScheduledEventId::new) else { return Ok(err("event_id is required")); };

        match guild_id.delete_scheduled_event(&self.http, event_id).await {
            Ok(()) => Ok(ok(json!({ "deleted": true }))),
            Err(error) => Ok(err(format!("Failed to delete scheduled event: {error}"))),
        }
    }
}
