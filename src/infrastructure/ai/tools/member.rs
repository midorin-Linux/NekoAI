use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::all::{GuildId, Http, UserId};

use super::error::DiscordToolError;

// ── GetMemberInfo ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GetMemberInfoArgs {
    guild_id: u64,
    user_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GetMemberInfo {
    #[serde(skip)]
    http: Option<Arc<Http>>,
}

impl GetMemberInfo {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http: Some(http) }
    }
}

impl Tool for GetMemberInfo {
    const NAME: &'static str = "get_member_info";
    type Error = DiscordToolError;
    type Args = GetMemberInfoArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "get_member_info".to_string(),
            description: "Get detailed information about a server member including nickname, roles, join date, etc.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    },
                    "user_id": {
                        "type": "integer",
                        "description": "The Discord user ID"
                    }
                },
                "required": ["guild_id", "user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("get_member_info", "HTTP client not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let user_id = UserId::new(args.user_id);

        let member = guild_id
            .member(http.as_ref(), user_id)
            .await
            .map_err(|e| DiscordToolError::not_found("get_member_info", e.to_string()))?;

        let roles_str: Vec<String> = member.roles.iter().map(|r| format!("{}", r)).collect();
        let joined_at = member
            .joined_at
            .map(|t| t.to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let info = format!(
            "User: {} (ID: {})\nNickname: {}\nRoles: [{}]\nJoined at: {}\nDeaf: {}\nMute: {}\nPending: {}",
            member.user.name,
            member.user.id,
            member.nick.as_deref().unwrap_or("None"),
            roles_str.join(", "),
            joined_at,
            member.deaf,
            member.mute,
            member.pending,
        );

        Ok(info)
    }
}

// ── SearchMembers ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SearchMembersArgs {
    guild_id: u64,
    query: String,
    #[serde(default = "default_limit")]
    limit: Option<u64>,
}

fn default_limit() -> Option<u64> {
    Some(10)
}

#[derive(Serialize, Deserialize)]
pub struct SearchMembers {
    #[serde(skip)]
    http: Option<Arc<Http>>,
}

impl SearchMembers {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http: Some(http) }
    }
}

impl Tool for SearchMembers {
    const NAME: &'static str = "search_members";
    type Error = DiscordToolError;
    type Args = SearchMembersArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "search_members".to_string(),
            description: "Search for members in a server by username or nickname.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query (username or nickname)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results to return (default: 10, max: 1000)"
                    }
                },
                "required": ["guild_id", "query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("search_members", "HTTP client not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let limit = args.limit.unwrap_or(10).min(1000);

        let members = guild_id
            .search_members(http.as_ref(), &args.query, Some(limit))
            .await
            .map_err(|e| DiscordToolError::api_error("search_members", e.to_string()))?;

        if members.is_empty() {
            return Ok(format!("No members found matching '{}'", args.query));
        }

        let mut result = format!("Members matching '{}':\n", args.query);
        for member in &members {
            let nick = member.nick.as_deref().unwrap_or("None");
            result.push_str(&format!(
                "- {} (ID: {}, nick: {})\n",
                member.user.name, member.user.id, nick
            ));
        }

        Ok(result)
    }
}
