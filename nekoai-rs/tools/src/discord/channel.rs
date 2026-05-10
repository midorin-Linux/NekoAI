use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{Value, json};
use serenity::{
    all::{CreateChannel, EditChannel, PermissionOverwrite, PermissionOverwriteType, Permissions},
    http::Http,
};

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, get_bool, get_channel_id, get_guild_id_default, get_string, get_u16, get_u32, get_u64,
        ok, parse_channel_type, retry_discord, to_value,
    },
};

pub struct CreateDiscordChannel {
    http: Arc<Http>,
}

pub struct ListChannels {
    inner: GetDiscordChannelList,
}

pub struct CreateChannelTool {
    inner: CreateDiscordChannel,
}

pub struct UpdateChannel {
    inner: ModifyDiscordChannel,
}

pub struct ArchiveChannel {
    http: Arc<Http>,
}

pub struct SetChannelPermissions {
    http: Arc<Http>,
}

impl ListChannels {
    pub fn new(http: Arc<Http>) -> Self {
        Self {
            inner: GetDiscordChannelList::new(http),
        }
    }
}

impl CreateChannelTool {
    pub fn new(http: Arc<Http>) -> Self {
        Self {
            inner: CreateDiscordChannel::new(http),
        }
    }
}

impl UpdateChannel {
    pub fn new(http: Arc<Http>) -> Self {
        Self {
            inner: ModifyDiscordChannel::new(http),
        }
    }
}

impl ArchiveChannel {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl SetChannelPermissions {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for ListChannels {
    const NAME: &'static str = "list_channels";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List all channels in a guild with category and kind info.".to_string(),
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
        self.inner.call(args).await
    }
}

impl Tool for CreateChannelTool {
    const NAME: &'static str = "create_channel";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Create a text, voice, stage, category, forum, or announcement channel."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer" },
                    "name": { "type": "string" },
                    "kind": { "type": "string" },
                    "topic": { "type": "string" },
                    "nsfw": { "type": "boolean" },
                    "parent_id": { "type": "integer" },
                    "position": { "type": "integer" },
                    "bitrate": { "type": "integer" },
                    "user_limit": { "type": "integer" },
                    "rate_limit_per_user": { "type": "integer" }
                },
                "required": ["guild_id", "name"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.inner.call(args).await
    }
}

impl Tool for UpdateChannel {
    const NAME: &'static str = "update_channel";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Update channel settings such as name, topic, slowmode, or parent."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer" },
                    "name": { "type": "string" },
                    "kind": { "type": "string" },
                    "topic": { "type": "string" },
                    "nsfw": { "type": "boolean" },
                    "parent_id": { "type": "integer" },
                    "position": { "type": "integer" },
                    "bitrate": { "type": "integer" },
                    "user_limit": { "type": "integer" },
                    "rate_limit_per_user": { "type": "integer" }
                },
                "required": ["channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        self.inner.call(args).await
    }
}

impl Tool for ArchiveChannel {
    const NAME: &'static str = "archive_channel";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Archive a channel by applying a read-only overwrite for @everyone."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id to archive." }
                },
                "required": ["channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        crate::admin_guard_channel!(&self.http, channel_id);

        let channel = match channel_id.to_channel(&self.http).await {
            Ok(channel) => channel,
            Err(error) => return Ok(err(format!("Failed to resolve channel: {error}"))),
        };

        let guild_id = match channel.guild() {
            Some(channel) => channel.guild_id,
            None => return Ok(err("This operation requires a guild channel.")),
        };

        let overwrite = PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL,
            deny: Permissions::SEND_MESSAGES | Permissions::SEND_TTS_MESSAGES,
            kind: PermissionOverwriteType::Role(serenity::all::RoleId::new(guild_id.get())),
        };

        match retry_discord(|| {
            let http = self.http.clone();
            let overwrite = overwrite.clone();
            async move { channel_id.create_permission(&http, overwrite).await }
        })
        .await
        {
            Ok(()) => Ok(ok(
                json!({ "archived": true, "channel_id": channel_id.get() }),
            )),
            Err(error) => Ok(err(format!("Failed to archive channel: {error}"))),
        }
    }
}

impl Tool for SetChannelPermissions {
    const NAME: &'static str = "set_channel_permissions";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Set or clear channel permission overrides for a role or user."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer" },
                    "target_type": { "type": "string", "enum": ["role", "user"] },
                    "target_id": { "type": "integer" },
                    "allow": { "type": "integer", "description": "Permission bits to allow." },
                    "deny": { "type": "integer", "description": "Permission bits to deny." },
                    "clear": { "type": "boolean", "description": "Delete the overwrite instead of setting it." }
                },
                "required": ["channel_id", "target_type", "target_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        crate::admin_guard_channel!(&self.http, channel_id);

        let Some(target_type) = get_string(&args, "target_type") else {
            return Ok(err("target_type is required"));
        };
        let Some(target_id) = get_u64(&args, "target_id") else {
            return Ok(err("target_id is required"));
        };

        let overwrite_target = match target_type.as_str() {
            "role" => PermissionOverwriteType::Role(serenity::all::RoleId::new(target_id)),
            "user" => PermissionOverwriteType::Member(serenity::all::UserId::new(target_id)),
            _ => return Ok(err("target_type must be 'role' or 'user'")),
        };

        if get_bool(&args, "clear").unwrap_or(false) {
            return match retry_discord(|| {
                let http = self.http.clone();
                async move { channel_id.delete_permission(&http, overwrite_target).await }
            })
            .await
            {
                Ok(()) => Ok(ok(json!({ "updated": true, "cleared": true }))),
                Err(error) => Ok(err(format!("Failed to clear overwrite: {error}"))),
            };
        }

        let allow_bits = get_u64(&args, "allow").unwrap_or(0);
        let deny_bits = get_u64(&args, "deny").unwrap_or(0);
        let overwrite = PermissionOverwrite {
            allow: Permissions::from_bits_truncate(allow_bits),
            deny: Permissions::from_bits_truncate(deny_bits),
            kind: overwrite_target,
        };

        match retry_discord(|| {
            let http = self.http.clone();
            let overwrite = overwrite.clone();
            async move { channel_id.create_permission(&http, overwrite).await }
        })
        .await
        {
            Ok(()) => Ok(ok(json!({ "updated": true, "cleared": false }))),
            Err(error) => Ok(err(format!("Failed to set overwrite: {error}"))),
        }
    }
}

pub struct DeleteDiscordChannel {
    http: Arc<Http>,
}

pub struct ModifyDiscordChannel {
    http: Arc<Http>,
}

pub struct GetDiscordChannelInfo {
    http: Arc<Http>,
}

pub struct GetDiscordChannelList {
    http: Arc<Http>,
}

impl CreateDiscordChannel {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl DeleteDiscordChannel {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl ModifyDiscordChannel {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl GetDiscordChannelInfo {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl GetDiscordChannelList {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for CreateDiscordChannel {
    const NAME: &'static str = "create_discord_channel";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Create a channel in a guild.".to_string(),
            parameters: json!({
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

        let mut builder = CreateChannel::new(name);

        if let Some(kind) = args.get("kind").and_then(parse_channel_type) {
            builder = builder.kind(kind);
        }
        if let Some(topic) = get_string(&args, "topic") {
            builder = builder.topic(topic);
        }
        if let Some(nsfw) = get_bool(&args, "nsfw") {
            builder = builder.nsfw(nsfw);
        }
        if let Some(parent_id) = get_channel_id(&args, "parent_id") {
            builder = builder.category(parent_id);
        }
        if let Some(position) = get_u16(&args, "position") {
            builder = builder.position(position);
        }
        if let Some(bitrate) = get_u32(&args, "bitrate") {
            builder = builder.bitrate(bitrate);
        }
        if let Some(user_limit) = get_u32(&args, "user_limit") {
            builder = builder.user_limit(user_limit);
        }
        if let Some(rate_limit) = get_u16(&args, "rate_limit_per_user") {
            builder = builder.rate_limit_per_user(rate_limit);
        }

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            let builder = builder.clone();
            async move { guild_id.create_channel(&http, builder).await }
        })
        .await
        {
            Ok(channel) => Ok(ok(to_value(&channel))),
            Err(error) => Ok(err(format!("Failed to create channel: {error}"))),
        }
    }
}

impl Tool for DeleteDiscordChannel {
    const NAME: &'static str = "delete_discord_channel";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Delete a channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id." }
                },
                "required": ["channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        crate::admin_guard_channel!(&self.http, channel_id);

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { channel_id.delete(&http).await }
        })
        .await
        {
            Ok(channel) => Ok(ok(to_value(&channel))),
            Err(error) => Ok(err(format!("Failed to delete channel: {error}"))),
        }
    }
}

impl Tool for ModifyDiscordChannel {
    const NAME: &'static str = "modify_discord_channel";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Modify channel settings.".to_string(),
            parameters: json!({
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
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        crate::admin_guard_channel!(&self.http, channel_id);

        let mut builder = EditChannel::new();
        let mut changed = false;

        if let Some(name) = get_string(&args, "name") {
            builder = builder.name(name);
            changed = true;
        }
        if let Some(kind) = args.get("kind").and_then(parse_channel_type) {
            builder = builder.kind(kind);
            changed = true;
        }
        if let Some(topic) = get_string(&args, "topic") {
            builder = builder.topic(topic);
            changed = true;
        }
        if let Some(nsfw) = get_bool(&args, "nsfw") {
            builder = builder.nsfw(nsfw);
            changed = true;
        }
        if let Some(parent_id) = get_channel_id(&args, "parent_id") {
            builder = builder.category(parent_id);
            changed = true;
        }
        if let Some(position) = get_u16(&args, "position") {
            builder = builder.position(position);
            changed = true;
        }
        if let Some(bitrate) = get_u32(&args, "bitrate") {
            builder = builder.bitrate(bitrate);
            changed = true;
        }
        if let Some(user_limit) = get_u32(&args, "user_limit") {
            builder = builder.user_limit(user_limit);
            changed = true;
        }
        if let Some(rate_limit) = get_u16(&args, "rate_limit_per_user") {
            builder = builder.rate_limit_per_user(rate_limit);
            changed = true;
        }

        if !changed {
            return Ok(err("No channel fields provided to modify"));
        }

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            let builder = builder.clone();
            async move { channel_id.edit(&http, builder).await }
        })
        .await
        {
            Ok(channel) => Ok(ok(to_value(&channel))),
            Err(error) => Ok(err(format!("Failed to modify channel: {error}"))),
        }
    }
}

impl Tool for GetDiscordChannelInfo {
    const NAME: &'static str = "get_discord_channel_info";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get channel information.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": { "type": "integer", "description": "Channel id." }
                },
                "required": ["channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { channel_id.to_channel(&http).await }
        })
        .await
        {
            Ok(channel) => Ok(ok(to_value(&channel))),
            Err(error) => Ok(err(format!("Failed to fetch channel info: {error}"))),
        }
    }
}

impl Tool for GetDiscordChannelList {
    const NAME: &'static str = "get_discord_channel_list";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get channel list.".to_string(),
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

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.channels(&http).await }
        })
        .await
        {
            Ok(channels) => {
                let channels = channels.values().cloned().collect::<Vec<_>>();
                Ok(ok(to_value(&channels)))
            }
            Err(error) => Ok(err(format!("Failed to fetch channel list: {error}"))),
        }
    }
}
