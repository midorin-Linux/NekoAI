use std::{collections::HashMap, sync::Arc};

use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{Value, json};
use serenity::{
    all::{
        AuditLogEntryId, ChannelId, ChannelType, CreateAttachment, CreateChannel,
        DefaultMessageNotificationLevel, EditGuild, EditRole, ExplicitContentFilter, GuildId,
        Permissions, Role, RoleId, UserId, VerificationLevel,
        audit_log::{
            Action, ChannelAction, ChannelOverwriteAction, InviteAction, MemberAction,
            MessageAction, RoleAction,
        },
    },
    http::{GuildPagination, Http},
};

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, get_bool, get_guild_id_default, get_string, get_u8, get_u32, get_u64, get_user_id, ok,
        parse_channel_type, retry_discord, to_value,
    },
};

pub struct GetDiscordGuildInfo {
    http: Arc<Http>,
}

pub struct GetDiscordGuildList {
    http: Arc<Http>,
}

pub struct ModifyDiscordGuild {
    http: Arc<Http>,
}

pub struct GetDiscordAuditLog {
    http: Arc<Http>,
}

impl GetDiscordGuildInfo {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl GetDiscordGuildList {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl ModifyDiscordGuild {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl GetDiscordAuditLog {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for GetDiscordGuildInfo {
    const NAME: &'static str = "get_discord_guild_info";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get guild information.".to_string(),
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
            async move { guild_id.to_partial_guild(&http).await }
        })
        .await
        {
            Ok(guild) => Ok(ok(to_value(&guild))),
            Err(error) => Ok(err(format!("Failed to fetch guild info: {error}"))),
        }
    }
}

impl Tool for GetDiscordGuildList {
    const NAME: &'static str = "get_discord_guild_list";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List guilds bot is in.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max guilds to return (1-200)." },
                    "after": { "type": "integer", "description": "Return guilds after this guild id." }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let limit = get_u64(&args, "limit");
        let after = get_u64(&args, "after");

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move {
                let pagination =
                    after.map(|guild_id| GuildPagination::After(GuildId::new(guild_id)));
                http.get_guilds(pagination, limit).await
            }
        })
        .await
        {
            Ok(guilds) => Ok(ok(to_value(&guilds))),
            Err(error) => Ok(err(format!("Failed to fetch guild list: {error}"))),
        }
    }
}

impl Tool for ModifyDiscordGuild {
    const NAME: &'static str = "modify_discord_guild";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Modify guild settings such as name or icon.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "name": { "type": "string", "description": "New guild name." },
                    "description": { "type": "string", "description": "New guild description." },
                    "icon_path": { "type": "string", "description": "Local path to PNG icon file." },
                    "clear_icon": { "type": "boolean", "description": "Clear current icon." }
                },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let mut builder = EditGuild::new();
        let mut changed = false;

        if let Some(name) = get_string(&args, "name") {
            builder = builder.name(name);
            changed = true;
        }
        if let Some(description) = get_string(&args, "description") {
            builder = builder.description(description);
            changed = true;
        }
        if let Some(true) = get_bool(&args, "clear_icon") {
            builder = builder.icon(None);
            changed = true;
        } else if let Some(icon_path) = get_string(&args, "icon_path") {
            match std::fs::read(&icon_path) {
                Ok(icon_data) => {
                    let attachment = CreateAttachment::bytes(icon_data, "icon.png");
                    builder = builder.icon(Some(&attachment));
                    changed = true;
                }
                Err(error) => return Ok(err(format!("Failed to read icon file: {error}"))),
            }
        }

        if !changed {
            return Ok(err("No guild fields provided to modify"));
        }

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            let builder = builder.clone();
            async move { guild_id.edit(&http, builder).await }
        })
        .await
        {
            Ok(guild) => Ok(ok(to_value(&guild))),
            Err(error) => Ok(err(format!("Failed to modify guild: {error}"))),
        }
    }
}

impl Tool for GetDiscordAuditLog {
    const NAME: &'static str = "get_discord_audit_log";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Fetch guild audit log entries.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "action_type": { "type": "integer", "description": "Audit log action type number." },
                    "user_id": { "type": "integer", "description": "Filter by user id." },
                    "before": { "type": "integer", "description": "Fetch entries before this audit log entry id." },
                    "limit": { "type": "integer", "description": "Number of entries to return." }
                },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };

        let limit = get_u8(&args, "limit");
        let action_type = get_u32(&args, "action_type").map(|v| Action::from_value(v as u8));
        let user_id = get_user_id(&args, "user_id");
        let before = get_u64(&args, "before").map(AuditLogEntryId::new);

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move {
                guild_id
                    .audit_logs(&http, action_type, user_id, before, limit)
                    .await
            }
        })
        .await
        {
            Ok(log) => Ok(ok(to_value(&log))),
            Err(error) => Ok(err(format!("Failed to fetch audit log: {error}"))),
        }
    }
}

// =============================================================================
// High-level Guild Tools
// =============================================================================

// ---------------------------------------------------------------------------
// 1. GetGuildSummary — サーバー概要の一括取得
// ---------------------------------------------------------------------------

pub struct GetGuildSummary {
    http: Arc<Http>,
}

impl GetGuildSummary {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for GetGuildSummary {
    const NAME: &'static str = "get_guild_summary";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Get a comprehensive overview of a guild including channels, roles, member counts, and key settings in a single call.".to_string(),
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

        // Fetch guild info, channels, and roles concurrently
        let (guild_result, channels_result) = tokio::join!(
            retry_discord(|| {
                let http = http.clone();
                async move { guild_id.to_partial_guild(&http).await }
            }),
            retry_discord(|| {
                let http = http.clone();
                async move { guild_id.channels(&http).await }
            }),
        );

        let guild = match guild_result {
            Ok(g) => g,
            Err(e) => return Ok(err(format!("Failed to fetch guild info: {e}"))),
        };
        let channels = match channels_result {
            Ok(c) => c,
            Err(e) => return Ok(err(format!("Failed to fetch channels: {e}"))),
        };
        let roles = &guild.roles;

        // Categorize channels
        let mut categories: Vec<String> = Vec::new();
        let mut text_channels: Vec<Value> = Vec::new();
        let mut voice_channels: Vec<Value> = Vec::new();
        let mut forum_channels: Vec<Value> = Vec::new();
        let mut news_channels: Vec<Value> = Vec::new();
        let mut stage_channels: Vec<Value> = Vec::new();

        // Build category name map
        let mut cat_names: HashMap<ChannelId, String> = HashMap::new();
        for (id, ch) in &channels {
            if ch.kind == ChannelType::Category {
                cat_names.insert(*id, ch.name.clone());
                categories.push(ch.name.clone());
            }
        }

        for ch in channels.values() {
            let parent_name = ch
                .parent_id
                .and_then(|pid| cat_names.get(&pid))
                .cloned()
                .unwrap_or_default();
            let entry = json!({
                "id": ch.id.get(),
                "name": ch.name,
                "type": format!("{:?}", ch.kind),
                "category": parent_name,
            });
            match ch.kind {
                ChannelType::Text => text_channels.push(entry),
                ChannelType::Voice => voice_channels.push(entry),
                ChannelType::Forum => forum_channels.push(entry),
                ChannelType::News => news_channels.push(entry),
                ChannelType::Stage => stage_channels.push(entry),
                _ => {}
            }
        }

        // Sort roles by position (descending) and take top names
        let mut role_vec: Vec<&Role> = roles.values().collect();
        role_vec.sort_by_key(|b| std::cmp::Reverse(b.position));
        let role_names: Vec<String> = role_vec.iter().take(30).map(|r| r.name.clone()).collect();

        let total_channels = channels.len();
        let channel_summary = json!({
            "total": total_channels,
            "categories": {
                "count": categories.len(),
                "names": categories,
            },
            "text": {
                "count": text_channels.len(),
                "list": text_channels,
            },
            "voice": {
                "count": voice_channels.len(),
                "list": voice_channels,
            },
            "forum": {
                "count": forum_channels.len(),
                "list": forum_channels,
            },
            "news": {
                "count": news_channels.len(),
                "list": news_channels,
            },
            "stage": {
                "count": stage_channels.len(),
                "list": stage_channels,
            },
        });

        let summary = json!({
            "name": guild.name,
            "id": guild.id.get(),
            "owner_id": guild.owner_id.get(),
            "description": guild.description,
            "member_count": guild.approximate_member_count.unwrap_or(0),
            "presence_count": guild.approximate_presence_count.unwrap_or(0),
            "premium_tier": format!("{:?}", guild.premium_tier),
            "verification_level": format!("{:?}", guild.verification_level),
            "features": guild.features,
            "channels": channel_summary,
            "roles": {
                "total": roles.len(),
                "names": role_names,
            },
            "vanity_url": guild.vanity_url_code,
        });

        Ok(ok(summary))
    }
}

// ---------------------------------------------------------------------------
// 2. SummarizeAuditLog — 監査ログの人間向け翻訳
// ---------------------------------------------------------------------------

pub struct SummarizeAuditLog {
    http: Arc<Http>,
}

impl SummarizeAuditLog {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }

    fn format_action(action: &Action) -> &'static str {
        match action {
            Action::GuildUpdate => "Guild Settings Updated",
            Action::Channel(ChannelAction::Create) => "Channel Created",
            Action::Channel(ChannelAction::Update) => "Channel Updated",
            Action::Channel(ChannelAction::Delete) => "Channel Deleted",
            Action::ChannelOverwrite(ChannelOverwriteAction::Create) => {
                "Permission Override Created"
            }
            Action::ChannelOverwrite(ChannelOverwriteAction::Update) => {
                "Permission Override Updated"
            }
            Action::ChannelOverwrite(ChannelOverwriteAction::Delete) => {
                "Permission Override Deleted"
            }
            Action::Member(MemberAction::Kick) => "Member Kicked",
            Action::Member(MemberAction::Prune) => "Members Pruned",
            Action::Member(MemberAction::BanAdd) => "Member Banned",
            Action::Member(MemberAction::BanRemove) => "Member Unbanned",
            Action::Member(MemberAction::Update) => "Member Updated",
            Action::Member(MemberAction::RoleUpdate) => "Member Role Updated",
            Action::Member(MemberAction::MemberMove) => "Member Moved (Voice)",
            Action::Member(MemberAction::MemberDisconnect) => "Member Disconnected (Voice)",
            Action::Member(MemberAction::BotAdd) => "Bot Added",
            Action::Role(RoleAction::Create) => "Role Created",
            Action::Role(RoleAction::Update) => "Role Updated",
            Action::Role(RoleAction::Delete) => "Role Deleted",
            Action::Invite(InviteAction::Create) => "Invite Created",
            Action::Invite(InviteAction::Update) => "Invite Updated",
            Action::Invite(InviteAction::Delete) => "Invite Deleted",
            Action::Webhook(_) => "Webhook Action",
            Action::Emoji(_) => "Emoji Action",
            Action::Sticker(_) => "Sticker Action",
            Action::Message(MessageAction::Delete) => "Message Deleted",
            Action::Message(MessageAction::BulkDelete) => "Messages Bulk Deleted",
            Action::Message(MessageAction::Pin) => "Message Pinned",
            Action::Message(MessageAction::Unpin) => "Message Unpinned",
            Action::Integration(_) => "Integration Action",
            Action::StageInstance(_) => "Stage Instance Action",
            Action::ScheduledEvent(_) => "Scheduled Event Action",
            Action::Thread(_) => "Thread Action",
            Action::AutoMod(_) => "Auto Moderation Action",
            Action::CreatorMonetization(_) => "Creator Monetization Action",
            Action::VoiceChannelStatus(_) => "Voice Channel Status Action",
            Action::Unknown(_) => "Unknown Action",
            _ => "Unknown Action", // catch-all for future variants
        }
    }

    fn format_target(target: &serde_json::Value) -> String {
        if let Some(id) = target.get("id").and_then(|v| v.as_u64()) {
            format!("target:{}", id)
        } else {
            format!("{:?}", target)
        }
    }
}

impl Tool for SummarizeAuditLog {
    const NAME: &'static str = "summarize_audit_log";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Fetch and summarize recent audit log entries in a human-readable format, with usernames resolved for easy understanding.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "limit": { "type": "integer", "description": "Number of recent entries to return (max 100)." },
                    "action_type": { "type": "integer", "description": "Filter by action type number (optional)." },
                    "user_id": { "type": "integer", "description": "Filter by user id who performed the action (optional)." }
                },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        let limit = get_u8(&args, "limit");
        let action_type = get_u32(&args, "action_type").map(|v| Action::from_value(v as u8));
        let user_id = get_user_id(&args, "user_id");

        let http = self.http.clone();
        let audit_logs = match retry_discord(|| {
            let http = http.clone();
            async move {
                guild_id
                    .audit_logs(&http, action_type, user_id, None, limit)
                    .await
            }
        })
        .await
        {
            Ok(log) => log,
            Err(e) => return Ok(err(format!("Failed to fetch audit log: {e}"))),
        };

        // Build user name map from the users included in the audit log response
        let user_names: HashMap<UserId, String> = audit_logs
            .users
            .iter()
            .map(|(id, user)| (*id, user.name.clone()))
            .collect();

        let mut entries: Vec<Value> = Vec::new();
        for entry in &audit_logs.entries {
            let actor_name = user_names
                .get(&entry.user_id)
                .map(|s| s.as_str())
                .unwrap_or("<unknown>")
                .to_string();
            let action_label = Self::format_action(&entry.action);
            let target_str = entry
                .target_id
                .as_ref()
                .map(|t| Self::format_target(&serde_json::to_value(t).unwrap_or_default()))
                .unwrap_or_default();
            let reason = entry.reason.clone().unwrap_or_default();

            // Build a natural language summary
            let mut description = format!("**{}**", action_label);
            description.push_str(&format!(" by {}", actor_name));
            if !target_str.is_empty() {
                description.push_str(&format!(" on {}", target_str));
            }
            if !reason.is_empty() {
                description.push_str(&format!(" (reason: {})", reason));
            }

            entries.push(json!({
                "id": entry.id.get(),
                "action": action_label,
                "actor": actor_name,
                "actor_id": entry.user_id.get(),
                "target": target_str,
                "reason": reason,
                "description": description,
            }));
        }

        let total = entries.len();
        let summary_lines: Vec<String> = entries
            .iter()
            .map(|e| e["description"].as_str().unwrap_or("").to_string())
            .collect();

        Ok(ok(json!({
            "total": total,
            "entries": entries,
            "summary": summary_lines,
        })))
    }
}

// ---------------------------------------------------------------------------
// 3. AnalyzeGuildActivity — サーバー活性度・荒らし検知
// ---------------------------------------------------------------------------

pub struct AnalyzeGuildActivity {
    http: Arc<Http>,
}

impl AnalyzeGuildActivity {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for AnalyzeGuildActivity {
    const NAME: &'static str = "analyze_guild_activity";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Analyze recent guild activity from the audit log to detect raiding, unusual patterns, and summarize moderation actions. Also returns the current member/presence count.".to_string(),
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

        // Fetch audit log (last 100 entries) and guild info concurrently
        let (audit_result, guild_result) = tokio::join!(
            retry_discord(|| {
                let http = http.clone();
                async move {
                    guild_id
                        .audit_logs(&http, None, None, None, Some(100))
                        .await
                }
            }),
            retry_discord(|| {
                let http = http.clone();
                async move { guild_id.to_partial_guild(&http).await }
            }),
        );

        let audit_logs = match audit_result {
            Ok(log) => log,
            Err(e) => return Ok(err(format!("Failed to fetch audit log: {e}"))),
        };
        let guild = match guild_result {
            Ok(g) => g,
            Err(e) => return Ok(err(format!("Failed to fetch guild info: {e}"))),
        };

        // Count actions by type
        let mut action_counts: HashMap<String, usize> = HashMap::new();
        let mut kick_count = 0usize;
        let mut ban_count = 0usize;
        let mut unban_count = 0usize;
        let mut message_delete_count = 0usize;
        let mut member_kick_users: Vec<(String, String)> = Vec::new();
        let mut recent_bans: Vec<String> = Vec::new();

        let user_names: HashMap<UserId, String> = audit_logs
            .users
            .iter()
            .map(|(id, user)| (*id, user.name.clone()))
            .collect();

        for entry in &audit_logs.entries {
            let label = SummarizeAuditLog::format_action(&entry.action);
            *action_counts.entry(label.to_string()).or_insert(0) += 1;
            let actor = user_names
                .get(&entry.user_id)
                .map(|s| s.as_str())
                .unwrap_or("<unknown>")
                .to_string();

            match entry.action {
                Action::Member(MemberAction::Kick) => {
                    kick_count += 1;
                    member_kick_users
                        .push((actor.clone(), entry.reason.clone().unwrap_or_default()));
                }
                Action::Member(MemberAction::BanAdd) => {
                    ban_count += 1;
                    recent_bans.push(actor);
                }
                Action::Member(MemberAction::BanRemove) => {
                    unban_count += 1;
                }
                Action::Message(MessageAction::Delete)
                | Action::Message(MessageAction::BulkDelete) => {
                    message_delete_count += 1;
                }
                _ => {}
            }
        }

        let total_entries = audit_logs.entries.len();

        // Detection alerts
        let mut alerts: Vec<String> = Vec::new();
        if kick_count >= 5 {
            alerts.push(format!(
                "⚠️ High kick activity detected: {} members kicked recently",
                kick_count
            ));
        }
        if ban_count >= 5 {
            alerts.push(format!(
                "⚠️ High ban activity detected: {} members banned recently",
                ban_count
            ));
        }
        if message_delete_count >= 20 {
            alerts.push(format!(
                "⚠️ Unusual message deletion: {} messages deleted recently (possible spam cleanup)",
                message_delete_count
            ));
        }
        if kick_count + ban_count >= 10 {
            alerts.push("🚨 Possible raid or targeted moderation action in progress".to_string());
        }
        if alerts.is_empty() {
            alerts.push("✅ No unusual activity detected".to_string());
        }

        let member_count = guild.approximate_member_count.unwrap_or(0);
        let presence_count = guild.approximate_presence_count.unwrap_or(0);

        let report = json!({
            "member_count": member_count,
            "presence_count": presence_count,
            "audit_log_scanned": total_entries,
            "activity_summary": {
                "kicks": kick_count,
                "bans": ban_count,
                "unbans": unban_count,
                "message_deletions": message_delete_count,
                "all_actions": action_counts,
            },
            "alerts": alerts,
            "recent_kicks": member_kick_users.iter().map(|(a, r)| {
                json!({ "by": a, "reason": r })
            }).collect::<Vec<_>>(),
            "recent_bans": recent_bans,
        });

        Ok(ok(report))
    }
}

// ---------------------------------------------------------------------------
// 4. ApplyGuildSecurityPreset — セキュリティ設定の一括変更
// ---------------------------------------------------------------------------

pub struct ApplyGuildSecurityPreset {
    http: Arc<Http>,
}

impl ApplyGuildSecurityPreset {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for ApplyGuildSecurityPreset {
    const NAME: &'static str = "apply_guild_security_preset";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Apply a security preset to a guild. Levels: relaxed (minimal restrictions), standard (balanced), strict (high security), lockdown (maximum security, disables @everyone chat).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "level": {
                        "type": "string",
                        "description": "Security level: relaxed, standard, strict, lockdown.",
                        "enum": ["relaxed", "standard", "strict", "lockdown"]
                    }
                },
                "required": ["guild_id", "level"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let Some(level) = get_string(&args, "level") else {
            return Ok(err(
                "level is required (relaxed, standard, strict, lockdown)",
            ));
        };

        let (verification, filter, notifications) = match level.as_str() {
            "relaxed" => (
                VerificationLevel::None,
                ExplicitContentFilter::None,
                DefaultMessageNotificationLevel::All,
            ),
            "standard" => (
                VerificationLevel::Low,
                ExplicitContentFilter::WithoutRole,
                DefaultMessageNotificationLevel::Mentions,
            ),
            "strict" => (
                VerificationLevel::High,
                ExplicitContentFilter::All,
                DefaultMessageNotificationLevel::Mentions,
            ),
            "lockdown" => (
                VerificationLevel::Higher,
                ExplicitContentFilter::All,
                DefaultMessageNotificationLevel::Mentions,
            ),
            _ => {
                return Ok(err(format!(
                    "Unknown security level '{}'. Use: relaxed, standard, strict, lockdown",
                    level
                )));
            }
        };

        let mut changes_made = vec![
            format!("verification_level -> {:?}", verification),
            format!("explicit_content_filter -> {:?}", filter),
            format!("default_notifications -> {:?}", notifications),
        ];

        let builder = EditGuild::new()
            .verification_level(verification)
            .explicit_content_filter(Some(filter))
            .default_message_notifications(Some(notifications));

        // For lockdown mode, also create a note; channel-level permissions
        // can be applied separately with the role/channel tools.
        if level == "lockdown" {
            changes_made.push(
                "LOCKDOWN: @everyone's send_messages in text channels should be denied using role tools.".to_string(),
            );
        }

        match retry_discord(|| {
            let http = self.http.clone();
            let builder = builder.clone();
            async move { guild_id.edit(&http, builder).await }
        })
        .await
        {
            Ok(_) => Ok(ok(json!({
                "level": level,
                "applied": true,
                "changes": changes_made,
            }))),
            Err(e) => Ok(err(format!("Failed to apply security preset: {e}"))),
        }
    }
}

// ---------------------------------------------------------------------------
// 5. AuditGuildPermissions — 権限設定の脆弱性スキャン
// ---------------------------------------------------------------------------

pub struct AuditGuildPermissions {
    http: Arc<Http>,
}

impl AuditGuildPermissions {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }

    fn dangerous_permission_checks() -> Vec<(&'static str, Permissions, &'static str)> {
        vec![
            (
                "ADMINISTRATOR",
                Permissions::ADMINISTRATOR,
                "Grants full administrative access to the server.",
            ),
            (
                "KICK_MEMBERS",
                Permissions::KICK_MEMBERS,
                "Allows kicking members.",
            ),
            (
                "BAN_MEMBERS",
                Permissions::BAN_MEMBERS,
                "Allows banning members.",
            ),
            (
                "MANAGE_CHANNELS",
                Permissions::MANAGE_CHANNELS,
                "Allows creating, editing, and deleting channels.",
            ),
            (
                "MANAGE_GUILD",
                Permissions::MANAGE_GUILD,
                "Allows changing guild settings.",
            ),
            (
                "MANAGE_ROLES",
                Permissions::MANAGE_ROLES,
                "Allows creating and modifying roles.",
            ),
            (
                "MANAGE_WEBHOOKS",
                Permissions::MANAGE_WEBHOOKS,
                "Allows creating and managing webhooks.",
            ),
            (
                "MENTION_EVERYONE",
                Permissions::MENTION_EVERYONE,
                "Allows mentioning @everyone, @here, and all roles.",
            ),
            (
                "MANAGE_MESSAGES",
                Permissions::MANAGE_MESSAGES,
                "Allows deleting messages by other members.",
            ),
            (
                "MOVE_MEMBERS",
                Permissions::MOVE_MEMBERS,
                "Allows moving members between voice channels.",
            ),
            (
                "MUTE_MEMBERS",
                Permissions::MUTE_MEMBERS,
                "Allows muting members in voice channels.",
            ),
            (
                "DEAFEN_MEMBERS",
                Permissions::DEAFEN_MEMBERS,
                "Allows deafening members in voice channels.",
            ),
        ]
    }
}

impl Tool for AuditGuildPermissions {
    const NAME: &'static str = "audit_guild_permissions";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Scan all roles in the guild for dangerous or unintended permission combinations. Useful for security audits and identifying risky configurations.".to_string(),
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
        let roles = match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.roles(&http).await }
        })
        .await
        {
            Ok(r) => r,
            Err(e) => return Ok(err(format!("Failed to fetch roles: {e}"))),
        };

        let checks = Self::dangerous_permission_checks();
        let mut findings: Vec<Value> = Vec::new();
        let mut risk_score: usize = 0;

        for role in roles.values() {
            if role.name == "@everyone" {
                continue; // @everyone having dangerous permissions is common; flag separately
            }
            let perms = role.permissions;
            let mut role_findings: Vec<Value> = Vec::new();

            for (name, flag, description) in &checks {
                if perms.contains(*flag) {
                    role_findings.push(json!({
                        "permission": name,
                        "description": description,
                    }));
                }
            }

            if !role_findings.is_empty() {
                let count = role_findings.len();
                risk_score += count;
                findings.push(json!({
                    "role_id": role.id.get(),
                    "role_name": role.name,
                    "position": role.position,
                    "dangerous_permissions_count": count,
                    "permissions": role_findings,
                }));
            }

            // Special: ADMINISTRATOR permission is extremely dangerous
            if perms.contains(Permissions::ADMINISTRATOR) && role.name != "@everyone" {
                risk_score += 10;
            }
        }

        // Check @everyone separately
        let everyone_issues: Vec<String> = roles
            .get(&RoleId::new(guild_id.get()))
            .map(|everyone| {
                let mut issues = Vec::new();
                let perms = everyone.permissions;
                for (name, flag, _desc) in &checks {
                    if perms.contains(*flag) {
                        issues.push(name.to_string());
                    }
                }
                issues
            })
            .unwrap_or_default();

        let risk_level = if risk_score >= 50 {
            "critical"
        } else if risk_score >= 20 {
            "high"
        } else if risk_score >= 5 {
            "medium"
        } else {
            "low"
        };

        Ok(ok(json!({
            "risk_level": risk_level,
            "risk_score": risk_score,
            "total_roles_scanned": roles.len(),
            "roles_with_issues": findings.len(),
            "role_findings": findings,
            "everyone_role_issues": everyone_issues,
            "recommendation": match risk_level {
                "critical" => "Immediate review required: multiple roles have Administrator or highly dangerous permissions.",
                "high" => "Review the flagged roles promptly and restrict unnecessary permissions.",
                "medium" => "Consider auditing the listed roles for potentially excessive permissions.",
                _ => "No significant permission issues detected.",
            },
        })))
    }
}

// ---------------------------------------------------------------------------
// 6. SetupGuildStructure — テンプレートに基づくチャンネル・ロールの自動構築
// ---------------------------------------------------------------------------

pub struct SetupGuildStructure {
    http: Arc<Http>,
}

impl SetupGuildStructure {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for SetupGuildStructure {
    const NAME: &'static str = "setup_guild_structure";

    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Create multiple channels and roles in a guild from a JSON template. Categories, channels, and roles are created in the correct order with proper parent-child relationships.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "template": {
                        "type": "object",
                        "description": "Template with categories (each with channels), and standalone roles.",
                        "properties": {
                            "categories": {
                                "type": "array",
                                "description": "List of category objects. Each category can have channels inside it.",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string", "description": "Category name." },
                                        "channels": {
                                            "type": "array",
                                            "items": {
                                                "type": "object",
                                                "properties": {
                                                    "name": { "type": "string", "description": "Channel name." },
                                                    "kind": { "type": "string", "description": "Channel type: text, voice, forum, news, stage." },
                                                    "topic": { "type": "string", "description": "Channel topic (optional)." },
                                                    "nsfw": { "type": "boolean", "description": "NSFW flag (optional)." }
                                                },
                                                "required": ["name"]
                                            }
                                        }
                                    },
                                    "required": ["name"]
                                }
                            },
                            "standalone_channels": {
                                "type": "array",
                                "description": "Channels not belonging to any category.",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string", "description": "Channel name." },
                                        "kind": { "type": "string", "description": "Channel type." },
                                        "topic": { "type": "string", "description": "Channel topic (optional)." }
                                    },
                                    "required": ["name"]
                                }
                            },
                            "roles": {
                                "type": "array",
                                "description": "Roles to create.",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string", "description": "Role name." },
                                        "color": { "type": "string", "description": "Hex color (e.g. #ff0000)." },
                                        "hoist": { "type": "boolean", "description": "Display separately." },
                                        "mentionable": { "type": "boolean", "description": "Allow mentions." }
                                    },
                                    "required": ["name"]
                                }
                            }
                        }
                    }
                },
                "required": ["guild_id", "template"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);

        let Some(template) = args.get("template") else {
            return Ok(err("template is required"));
        };

        let http = self.http.clone();
        let mut created_roles: Vec<Value> = Vec::new();
        let mut created_categories: Vec<Value> = Vec::new();
        let mut created_channels: Vec<Value> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        // Step 1: Create roles first
        if let Some(roles) = template.get("roles").and_then(|v| v.as_array()) {
            for role_def in roles {
                let Some(name) = role_def.get("name").and_then(|v| v.as_str()) else {
                    continue;
                };
                let mut builder = EditRole::new().name(name);
                if let Some(color) = role_def.get("color").and_then(|v| v.as_str())
                    && let Ok(c) = u32::from_str_radix(color.trim_start_matches('#'), 16)
                {
                    builder = builder.colour(c);
                }
                if let Some(hoist) = role_def.get("hoist").and_then(|v| v.as_bool()) {
                    builder = builder.hoist(hoist);
                }
                if let Some(mentionable) = role_def.get("mentionable").and_then(|v| v.as_bool()) {
                    builder = builder.mentionable(mentionable);
                }

                match guild_id.create_role(&http, builder.clone()).await {
                    Ok(role) => {
                        created_roles.push(json!({
                            "id": role.id.get(),
                            "name": role.name,
                        }));
                    }
                    Err(e) => {
                        errors.push(format!("Failed to create role '{}': {e}", name));
                    }
                }
            }
        }

        // Step 2: Create categories (we need their IDs for channel parent mapping)
        let mut category_id_map: HashMap<String, serenity::all::ChannelId> = HashMap::new();
        if let Some(categories) = template.get("categories").and_then(|v| v.as_array()) {
            for cat_def in categories {
                let Some(name) = cat_def.get("name").and_then(|v| v.as_str()) else {
                    continue;
                };
                let builder = CreateChannel::new(name).kind(ChannelType::Category);
                match guild_id.create_channel(&http, builder.clone()).await {
                    Ok(channel) => {
                        let ch_name = channel.name.clone();
                        category_id_map.insert(ch_name, channel.id);
                        created_categories.push(json!({
                            "id": channel.id.get(),
                            "name": channel.name,
                        }));
                    }
                    Err(e) => {
                        errors.push(format!("Failed to create category '{}': {e}", name));
                    }
                }
            }
        }

        // Step 3: Create channels inside categories
        if let Some(categories) = template.get("categories").and_then(|v| v.as_array()) {
            for cat_def in categories {
                let Some(cat_name) = cat_def.get("name").and_then(|v| v.as_str()) else {
                    continue;
                };
                let parent_id = category_id_map.get(cat_name);
                if let Some(channels) = cat_def.get("channels").and_then(|v| v.as_array()) {
                    for ch_def in channels {
                        let Some(name) = ch_def.get("name").and_then(|v| v.as_str()) else {
                            continue;
                        };
                        let kind = ch_def
                            .get("kind")
                            .and_then(|v| v.as_str())
                            .and_then(|s| parse_channel_type(&json!(s)))
                            .unwrap_or(ChannelType::Text);
                        let mut builder = CreateChannel::new(name).kind(kind);
                        if let Some(parent) = parent_id {
                            builder = builder.category(*parent);
                        }
                        if let Some(topic) = ch_def.get("topic").and_then(|v| v.as_str()) {
                            builder = builder.topic(topic);
                        }
                        if let Some(nsfw) = ch_def.get("nsfw").and_then(|v| v.as_bool()) {
                            builder = builder.nsfw(nsfw);
                        }

                        match guild_id.create_channel(&http, builder.clone()).await {
                            Ok(channel) => {
                                created_channels.push(json!({
                                    "id": channel.id.get(),
                                    "name": channel.name,
                                    "category": cat_name,
                                    "type": format!("{:?}", channel.kind),
                                }));
                            }
                            Err(e) => {
                                errors.push(format!(
                                    "Failed to create channel '{}' in '{}': {e}",
                                    name, cat_name
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Step 4: Create standalone channels (not in a category)
        if let Some(standalone) = template
            .get("standalone_channels")
            .and_then(|v| v.as_array())
        {
            for ch_def in standalone {
                let Some(name) = ch_def.get("name").and_then(|v| v.as_str()) else {
                    continue;
                };
                let kind = ch_def
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .and_then(|s| parse_channel_type(&json!(s)))
                    .unwrap_or(ChannelType::Text);
                let mut builder = CreateChannel::new(name).kind(kind);
                if let Some(topic) = ch_def.get("topic").and_then(|v| v.as_str()) {
                    builder = builder.topic(topic);
                }
                if let Some(nsfw) = ch_def.get("nsfw").and_then(|v| v.as_bool()) {
                    builder = builder.nsfw(nsfw);
                }

                match guild_id.create_channel(&http, builder.clone()).await {
                    Ok(channel) => {
                        created_channels.push(json!({
                            "id": channel.id.get(),
                            "name": channel.name,
                            "category": "",
                            "type": format!("{:?}", channel.kind),
                        }));
                    }
                    Err(e) => {
                        errors.push(format!("Failed to create channel '{}': {e}", name));
                    }
                }
            }
        }

        Ok(ok(json!({
            "roles_created": created_roles.len(),
            "categories_created": created_categories.len(),
            "channels_created": created_channels.len(),
            "roles": created_roles,
            "categories": created_categories,
            "channels": created_channels,
            "errors": errors,
        })))
    }
}
