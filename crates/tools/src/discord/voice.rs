use std::sync::Arc;

use futures::future::join_all;
use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{Value, json};
use serenity::{
    all::{ChannelId, EditMember, GuildId, RoleId, UserId, VoiceState},
    http::{Http, LightMethod, Request, Route},
};

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, get_bool, get_channel_id, get_guild_id_default, get_u64, get_u64_list, get_user_id,
        ok, retry_discord, to_value,
    },
};

// ---------------------------------------------------------------------------
// Tool structs
// ---------------------------------------------------------------------------

/// Fetch the current voice state snapshot of all channels in a guild.
pub struct GetVoiceChannelStates {
    http: Arc<Http>,
}

/// Move every member in a source voice channel to a target voice channel.
pub struct MoveAllVoiceMembers {
    http: Arc<Http>,
}

/// Disconnect every member from a voice channel.
pub struct DisconnectAllVoiceMembers {
    http: Arc<Http>,
}

/// Mute (or unmute) all members in a voice channel, with optional exclusions.
pub struct SetChannelMuteState {
    http: Arc<Http>,
}

/// Move a list of specific members into a target voice channel.
pub struct BulkMoveVoiceMembers {
    http: Arc<Http>,
}

/// Change mute / deafen state for a list of specific members.
pub struct BulkSetMembersVoiceState {
    http: Arc<Http>,
}

/// Gather all members with a given role who are currently in any voice channel
/// and move them into the specified channel.
pub struct GatherMembersByRoleVoice {
    http: Arc<Http>,
}

// ---------------------------------------------------------------------------
// Keep low-level wrappers for backward compatibility (they delegate to the
// high-level spirit but are preserved so existing callers / agent
// configurations do not break).
// ---------------------------------------------------------------------------

pub struct MoveDiscordMemberVoice {
    http: Arc<Http>,
}
pub struct DisconnectDiscordMemberVoice {
    http: Arc<Http>,
}
pub struct MuteDiscordMember {
    http: Arc<Http>,
}
pub struct DeafenDiscordMember {
    http: Arc<Http>,
}

// ---------------------------------------------------------------------------
// Constructors — new
// ---------------------------------------------------------------------------

impl GetVoiceChannelStates {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl MoveAllVoiceMembers {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl DisconnectAllVoiceMembers {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl SetChannelMuteState {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl BulkMoveVoiceMembers {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl BulkSetMembersVoiceState {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl GatherMembersByRoleVoice {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl MoveDiscordMemberVoice {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl DisconnectDiscordMemberVoice {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl MuteDiscordMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}
impl DeafenDiscordMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

// ===========================================================================
// Internal helpers
// ===========================================================================

/// Fetch the full `VoiceState` list for every user in the guild.
///
/// Makes a raw REST request to `GET /guilds/{guild.id}` and extracts the
/// `voice_states` field from the response (serenity's `PartialGuild` does not
/// expose this field, but the underlying Discord API does return it).
async fn fetch_voice_states(
    http: &Http,
    guild_id: GuildId,
) -> Result<Vec<VoiceState>, DiscordToolError> {
    let req = Request::new(Route::Guild { guild_id }, LightMethod::Get);
    let json: serde_json::Value = http.fire(req).await?;
    let states: Vec<VoiceState> = serde_json::from_value(json["voice_states"].clone())?;
    Ok(states)
}

/// Return only voice states whose `channel_id` matches the given channel.
fn voice_states_in_channel(states: &[VoiceState], channel_id: ChannelId) -> Vec<&VoiceState> {
    states
        .iter()
        .filter(|vs| vs.channel_id == Some(channel_id))
        .collect()
}

/// Build a summary result for a bulk operation.
fn bulk_result(total: usize, succeeded: usize, details: Vec<Value>) -> Value {
    ok(json!({
        "total": total,
        "succeeded": succeeded,
        "failed": total.saturating_sub(succeeded),
        "details": details
    }))
}

fn detail_ok(user_id: UserId, action: &str) -> Value {
    json!({ "user_id": user_id.get(), "status": action })
}

fn detail_err(user_id: UserId, error: &str) -> Value {
    json!({ "user_id": user_id.get(), "error": error })
}

// ===========================================================================
// 1) GetVoiceChannelStates — read-only snapshot
// ===========================================================================

impl Tool for GetVoiceChannelStates {
    const NAME: &'static str = "get_voice_channel_states";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get a snapshot of all voice channels and who is in them, including mute/deafen status and roles.".to_string(),
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

        let states = match fetch_voice_states(&self.http, guild_id).await {
            Ok(s) => s,
            Err(e) => return Ok(err(format!("Failed to fetch voice states: {e}"))),
        };

        // Group by channel_id
        let mut channels: Vec<Value> = Vec::new();
        let mut seen = std::collections::BTreeSet::new();

        for vs in &states {
            let cid = match vs.channel_id {
                Some(c) => c,
                None => continue,
            };
            if !seen.insert(cid) {
                continue;
            }
            let members_in_channel: Vec<Value> = states
                .iter()
                .filter(|s| s.channel_id == Some(cid))
                .map(|s| {
                    let roles: Vec<u64> = s
                        .member
                        .as_ref()
                        .map(|m| m.roles.iter().map(|r| r.get()).collect())
                        .unwrap_or_default();
                    json!({
                        "user_id": s.user_id.get(),
                        "mute": s.mute,
                        "deafen": s.deaf,
                        "self_mute": s.self_mute,
                        "self_deafen": s.self_deaf,
                        "suppress": s.suppress,
                        "roles": roles
                    })
                })
                .collect();

            channels.push(json!({
                "channel_id": cid.get(),
                "member_count": members_in_channel.len(),
                "members": members_in_channel
            }));
        }

        Ok(ok(json!({
            "guild_id": guild_id.get(),
            "total_voice_members": states.len(),
            "channels": channels
        })))
    }
}

// ===========================================================================
// 2) MoveAllVoiceMembers — channel-based bulk move
// ===========================================================================

impl Tool for MoveAllVoiceMembers {
    const NAME: &'static str = "move_all_voice_members";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Move ALL members from one voice channel to another voice channel."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "from_channel_id": { "type": "integer", "description": "Source voice channel id." },
                    "to_channel_id": { "type": "integer", "description": "Target voice channel id." }
                },
                "required": ["guild_id", "from_channel_id", "to_channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let Some(from_channel) = get_channel_id(&args, "from_channel_id") else {
            return Ok(err("from_channel_id is required"));
        };
        let Some(to_channel) = get_channel_id(&args, "to_channel_id") else {
            return Ok(err("to_channel_id is required"));
        };

        let states = match fetch_voice_states(&self.http, guild_id).await {
            Ok(s) => s,
            Err(e) => return Ok(err(format!("Failed to fetch voice states: {e}"))),
        };

        let targets: Vec<UserId> = voice_states_in_channel(&states, from_channel)
            .into_iter()
            .map(|vs| vs.user_id)
            .collect();

        if targets.is_empty() {
            return Ok(ok(json!({
                "message": "No members found in the source channel.",
                "total": 0,
                "succeeded": 0
            })));
        }

        let (succeeded, details) =
            bulk_move_users(&self.http, guild_id, &targets, to_channel).await;
        Ok(bulk_result(targets.len(), succeeded, details))
    }
}

// ===========================================================================
// 3) DisconnectAllVoiceMembers — channel-based bulk disconnect
// ===========================================================================

impl Tool for DisconnectAllVoiceMembers {
    const NAME: &'static str = "disconnect_all_voice_members";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Disconnect (kick) ALL members from a voice channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "channel_id": { "type": "integer", "description": "Voice channel id." }
                },
                "required": ["guild_id", "channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };

        let states = match fetch_voice_states(&self.http, guild_id).await {
            Ok(s) => s,
            Err(e) => return Ok(err(format!("Failed to fetch voice states: {e}"))),
        };

        let targets: Vec<UserId> = voice_states_in_channel(&states, channel_id)
            .into_iter()
            .map(|vs| vs.user_id)
            .collect();

        if targets.is_empty() {
            return Ok(ok(json!({
                "message": "No members found in the channel.",
                "total": 0,
                "succeeded": 0
            })));
        }

        let (succeeded, details) = bulk_disconnect_users(&self.http, guild_id, &targets).await;
        Ok(bulk_result(targets.len(), succeeded, details))
    }
}

// ===========================================================================
// 4) SetChannelMuteState — channel-wide mute/deafen with exclude list
// ===========================================================================

impl Tool for SetChannelMuteState {
    const NAME: &'static str = "set_channel_mute_state";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Mute or unmute ALL members in a voice channel. Optionally exclude specific users from the change. Also supports deafen.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "channel_id": { "type": "integer", "description": "Voice channel id." },
                    "mute": { "type": "boolean", "description": "Server-mute (true) or unmute (false)." },
                    "deafen": { "type": "boolean", "description": "Server-deafen (true) or undeafen (false). Optional." },
                    "exclude_user_ids": { "type": "array", "items": { "type": "integer" }, "description": "User ids to exclude from the change. Optional." }
                },
                "required": ["guild_id", "channel_id", "mute"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };
        let Some(mute) = get_bool(&args, "mute") else {
            return Ok(err("mute is required"));
        };
        let deafen = get_bool(&args, "deafen");
        let exclude: std::collections::HashSet<u64> = get_u64_list(&args, "exclude_user_ids")
            .unwrap_or_default()
            .into_iter()
            .collect();

        let states = match fetch_voice_states(&self.http, guild_id).await {
            Ok(s) => s,
            Err(e) => return Ok(err(format!("Failed to fetch voice states: {e}"))),
        };

        let targets: Vec<UserId> = voice_states_in_channel(&states, channel_id)
            .into_iter()
            .filter(|vs| !exclude.contains(&vs.user_id.get()))
            .map(|vs| vs.user_id)
            .collect();

        if targets.is_empty() {
            return Ok(ok(json!({
                "message": "No applicable members found in the channel (all may be excluded).",
                "total": 0,
                "succeeded": 0
            })));
        }

        let (succeeded, details) =
            bulk_set_voice_state(&self.http, guild_id, &targets, Some(mute), deafen).await;
        Ok(bulk_result(targets.len(), succeeded, details))
    }
}

// ===========================================================================
// 5) BulkMoveVoiceMembers — list-based bulk move
// ===========================================================================

impl Tool for BulkMoveVoiceMembers {
    const NAME: &'static str = "bulk_move_voice_members";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Move a specific list of members into a target voice channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_ids": { "type": "array", "items": { "type": "integer" }, "description": "User ids to move." },
                    "to_channel_id": { "type": "integer", "description": "Target voice channel id." }
                },
                "required": ["guild_id", "user_ids", "to_channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let Some(user_ids) = get_u64_list(&args, "user_ids") else {
            return Ok(err("user_ids is required"));
        };
        let Some(to_channel) = get_channel_id(&args, "to_channel_id") else {
            return Ok(err("to_channel_id is required"));
        };

        let targets: Vec<UserId> = user_ids.into_iter().map(UserId::new).collect();
        if targets.is_empty() {
            return Ok(err("user_ids must not be empty"));
        }

        let (succeeded, details) =
            bulk_move_users(&self.http, guild_id, &targets, to_channel).await;
        Ok(bulk_result(targets.len(), succeeded, details))
    }
}

// ===========================================================================
// 6) BulkSetMembersVoiceState — list-based mute / deafen
// ===========================================================================

impl Tool for BulkSetMembersVoiceState {
    const NAME: &'static str = "bulk_set_members_voice_state";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Change mute and/or deafen state for a specific list of members."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_ids": { "type": "array", "items": { "type": "integer" }, "description": "User ids to modify." },
                    "mute": { "type": "boolean", "description": "Server-mute (true) or unmute (false). Optional." },
                    "deafen": { "type": "boolean", "description": "Server-deafen (true) or undeafen (false). Optional." }
                },
                "required": ["guild_id", "user_ids"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let Some(user_ids) = get_u64_list(&args, "user_ids") else {
            return Ok(err("user_ids is required"));
        };
        let mute = get_bool(&args, "mute");
        let deafen = get_bool(&args, "deafen");

        if mute.is_none() && deafen.is_none() {
            return Ok(err("At least one of 'mute' or 'deafen' is required"));
        }

        let targets: Vec<UserId> = user_ids.into_iter().map(UserId::new).collect();
        if targets.is_empty() {
            return Ok(err("user_ids must not be empty"));
        }

        let (succeeded, details) =
            bulk_set_voice_state(&self.http, guild_id, &targets, mute, deafen).await;
        Ok(bulk_result(targets.len(), succeeded, details))
    }
}

// ===========================================================================
// 7) GatherMembersByRoleVoice — role-based gather
// ===========================================================================

impl Tool for GatherMembersByRoleVoice {
    const NAME: &'static str = "gather_members_by_role_voice";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Find all members with a specific role who are currently in any voice channel and move them to a target channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "role_id": { "type": "integer", "description": "Role id to filter by." },
                    "to_channel_id": { "type": "integer", "description": "Target voice channel id." }
                },
                "required": ["guild_id", "role_id", "to_channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let role_id = match get_u64(&args, "role_id") {
            Some(id) => RoleId::new(id),
            None => return Ok(err("role_id is required")),
        };
        let Some(to_channel) = get_channel_id(&args, "to_channel_id") else {
            return Ok(err("to_channel_id is required"));
        };

        let states = match fetch_voice_states(&self.http, guild_id).await {
            Ok(s) => s,
            Err(e) => return Ok(err(format!("Failed to fetch voice states: {e}"))),
        };

        // Filter: must be in a voice channel AND have the target role.
        let targets: Vec<UserId> = states
            .iter()
            .filter(|vs| {
                vs.channel_id.is_some()
                    && vs
                        .member
                        .as_ref()
                        .is_some_and(|m| m.roles.contains(&role_id))
            })
            .map(|vs| vs.user_id)
            .collect();

        if targets.is_empty() {
            return Ok(ok(json!({
                "message": "No members with that role found in voice channels.",
                "total": 0,
                "succeeded": 0
            })));
        }

        let (succeeded, details) =
            bulk_move_users(&self.http, guild_id, &targets, to_channel).await;
        Ok(bulk_result(targets.len(), succeeded, details))
    }
}

// ===========================================================================
// Low-level backward-compatible wrappers
// ===========================================================================

impl Tool for MoveDiscordMemberVoice {
    const NAME: &'static str = "move_discord_member_voice";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Move a member to a voice channel.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." },
                    "channel_id": { "type": "integer", "description": "Target voice channel id." }
                },
                "required": ["guild_id", "user_id", "channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);
        let Some(user_id) = get_user_id(&args, "user_id") else {
            return Ok(err("user_id is required"));
        };
        let Some(channel_id) = get_channel_id(&args, "channel_id") else {
            return Ok(err("channel_id is required"));
        };

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.move_member(&http, user_id, channel_id).await }
        })
        .await
        {
            Ok(member) => Ok(ok(to_value(&member))),
            Err(error) => Ok(err(format!("Failed to move member: {error}"))),
        }
    }
}

impl Tool for DisconnectDiscordMemberVoice {
    const NAME: &'static str = "disconnect_discord_member_voice";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Disconnect a member from voice.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." }
                },
                "required": ["guild_id", "user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);
        let Some(user_id) = get_user_id(&args, "user_id") else {
            return Ok(err("user_id is required"));
        };

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.disconnect_member(&http, user_id).await }
        })
        .await
        {
            Ok(member) => Ok(ok(to_value(&member))),
            Err(error) => Ok(err(format!("Failed to disconnect member: {error}"))),
        }
    }
}

impl Tool for MuteDiscordMember {
    const NAME: &'static str = "mute_discord_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Server mute a member.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." },
                    "mute": { "type": "boolean", "description": "Mute flag." }
                },
                "required": ["guild_id", "user_id", "mute"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);
        let Some(user_id) = get_user_id(&args, "user_id") else {
            return Ok(err("user_id is required"));
        };
        let Some(mute) = get_bool(&args, "mute") else {
            return Ok(err("mute is required"));
        };

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move {
                guild_id
                    .edit_member(&http, user_id, EditMember::new().mute(mute))
                    .await
            }
        })
        .await
        {
            Ok(member) => Ok(ok(to_value(&member))),
            Err(error) => Ok(err(format!("Failed to mute member: {error}"))),
        }
    }
}

impl Tool for DeafenDiscordMember {
    const NAME: &'static str = "deafen_discord_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Server deafen a member.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." },
                    "deafen": { "type": "boolean", "description": "Deafen flag." }
                },
                "required": ["guild_id", "user_id", "deafen"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);
        let Some(user_id) = get_user_id(&args, "user_id") else {
            return Ok(err("user_id is required"));
        };
        let Some(deafen) = get_bool(&args, "deafen") else {
            return Ok(err("deafen is required"));
        };

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move {
                guild_id
                    .edit_member(&http, user_id, EditMember::new().deafen(deafen))
                    .await
            }
        })
        .await
        {
            Ok(member) => Ok(ok(to_value(&member))),
            Err(error) => Ok(err(format!("Failed to deafen member: {error}"))),
        }
    }
}

// ===========================================================================
// Bulk-operation primitives (shared by high-level tools)
// ===========================================================================

/// Move multiple users into a voice channel. Runs all moves concurrently.
async fn bulk_move_users(
    http: &Http,
    guild_id: GuildId,
    user_ids: &[UserId],
    channel_id: ChannelId,
) -> (usize, Vec<Value>) {
    let futs: Vec<_> = user_ids
        .iter()
        .map(|&uid| async move {
            retry_discord(|| async move { guild_id.move_member(http, uid, channel_id).await })
                .await
                .map(|_member| detail_ok(uid, "moved"))
                .unwrap_or_else(|e| detail_err(uid, &format!("{e}")))
        })
        .collect();

    let results = join_all(futs).await;
    let succeeded = results.iter().filter(|r| r.get("status").is_some()).count();
    (succeeded, results)
}

/// Disconnect multiple users from voice concurrently.
async fn bulk_disconnect_users(
    http: &Http,
    guild_id: GuildId,
    user_ids: &[UserId],
) -> (usize, Vec<Value>) {
    let futs: Vec<_> = user_ids
        .iter()
        .map(|&uid| async move {
            retry_discord(|| async move { guild_id.disconnect_member(http, uid).await })
                .await
                .map(|_member| detail_ok(uid, "disconnected"))
                .unwrap_or_else(|e| detail_err(uid, &format!("{e}")))
        })
        .collect();

    let results = join_all(futs).await;
    let succeeded = results.iter().filter(|r| r.get("status").is_some()).count();
    (succeeded, results)
}

/// Set mute and/or deafen for multiple users concurrently.
async fn bulk_set_voice_state(
    http: &Http,
    guild_id: GuildId,
    user_ids: &[UserId],
    mute: Option<bool>,
    deafen: Option<bool>,
) -> (usize, Vec<Value>) {
    let futs: Vec<_> = user_ids
        .iter()
        .map(|&uid| {
            let mut builder = EditMember::new();
            if let Some(m) = mute {
                builder = builder.mute(m);
            }
            if let Some(d) = deafen {
                builder = builder.deafen(d);
            }

            async move {
                retry_discord(|| {
                    let builder = builder.clone();
                    async move { guild_id.edit_member(http, uid, builder).await }
                })
                .await
                .map(|_member| detail_ok(uid, "updated"))
                .unwrap_or_else(|e| detail_err(uid, &format!("{e}")))
            }
        })
        .collect();

    let results = join_all(futs).await;
    let succeeded = results.iter().filter(|r| r.get("status").is_some()).count();
    (succeeded, results)
}
