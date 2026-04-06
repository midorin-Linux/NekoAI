use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::all::{Cache, GuildId, Http, Permissions, RoleId, UserId};

use super::error::{DiscordToolError, require_user_permission};

// ── ListRoles ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListRolesArgs {
    guild_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct ListRoles {
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl ListRoles {
    pub fn new(cache: Arc<Cache>) -> Self {
        Self { cache: Some(cache) }
    }
}

impl Tool for ListRoles {
    const NAME: &'static str = "list_roles";
    type Error = DiscordToolError;
    type Args = ListRolesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "list_roles".to_string(),
            description: "List all roles in a Discord server.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    }
                },
                "required": ["guild_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("list_roles", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let guild = cache
            .guild(guild_id)
            .ok_or_else(|| DiscordToolError::not_found("list_roles", "Guild not found in cache"))?;

        let mut result = format!("Roles in guild {}:\n", guild.name);
        let mut roles: Vec<_> = guild.roles.values().cloned().collect();
        roles.sort_by(|a, b| b.position.cmp(&a.position));

        for role in &roles {
            let member_count_str = if role.name == "@everyone" {
                "all members".to_string()
            } else {
                format!("color: #{:06x}", role.colour.0)
            };
            result.push_str(&format!(
                "- @{} (ID: {}, position: {}, {})\n",
                role.name, role.id, role.position, member_count_str
            ));
        }

        Ok(result)
    }
}

// ── GetRoleInfo ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GetRoleInfoArgs {
    guild_id: u64,
    role_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GetRoleInfo {
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl GetRoleInfo {
    pub fn new(cache: Arc<Cache>) -> Self {
        Self { cache: Some(cache) }
    }
}

impl Tool for GetRoleInfo {
    const NAME: &'static str = "get_role_info";
    type Error = DiscordToolError;
    type Args = GetRoleInfoArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "get_role_info".to_string(),
            description: "Get detailed information about a specific role.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": {
                        "type": "integer",
                        "description": "The Discord guild (server) ID"
                    },
                    "role_id": {
                        "type": "integer",
                        "description": "The Discord role ID"
                    }
                },
                "required": ["guild_id", "role_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("get_role_info", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let guild = cache.guild(guild_id).ok_or_else(|| {
            DiscordToolError::not_found("get_role_info", "Guild not found in cache")
        })?;

        let role_id = RoleId::new(args.role_id);
        let role = guild.roles.get(&role_id).ok_or_else(|| {
            DiscordToolError::not_found("get_role_info", format!("Role {} not found", args.role_id))
        })?;

        let info = format!(
            "Role: @{}\nID: {}\nColor: #{:06x}\nPosition: {}\nHoisted: {}\nMentionable: {}\nManaged: {}\nPermissions: {:?}",
            role.name,
            role.id,
            role.colour.0,
            role.position,
            role.hoist,
            role.mentionable,
            role.managed,
            role.permissions,
        );

        Ok(info)
    }
}

// ── AssignRole ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AssignRoleArgs {
    guild_id: u64,
    user_id: u64,
    role_id: u64,
    /// この操作を指示したユーザーのID（メタデータから取得）
    requesting_user_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct AssignRole {
    #[serde(skip)]
    http: Option<Arc<Http>>,
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl AssignRole {
    pub fn new(http: Arc<Http>, cache: Arc<Cache>) -> Self {
        Self {
            http: Some(http),
            cache: Some(cache),
        }
    }
}

impl Tool for AssignRole {
    const NAME: &'static str = "assign_role";
    type Error = DiscordToolError;
    type Args = AssignRoleArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "assign_role".to_string(),
            description:
                "Assign a role to a user. The requesting user must have MANAGE_ROLES permission."
                    .to_string(),
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
                    },
                    "role_id": {
                        "type": "integer",
                        "description": "The Discord role ID to assign"
                    },
                    "requesting_user_id": {
                        "type": "integer",
                        "description": "The user ID of the person who requested this action (from metadata)"
                    }
                },
                "required": ["guild_id", "user_id", "role_id", "requesting_user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("assign_role", "HTTP client not available"))?;
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("assign_role", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let user_id = UserId::new(args.user_id);
        let role_id = RoleId::new(args.role_id);
        let requesting_user_id = UserId::new(args.requesting_user_id);

        // MANAGE_ROLES 権限チェック
        require_user_permission(
            cache,
            http,
            guild_id,
            requesting_user_id,
            Permissions::MANAGE_ROLES,
            "assign_role",
        )
        .await?;

        let member = guild_id
            .member(http.as_ref(), user_id)
            .await
            .map_err(|e| DiscordToolError::not_found("assign_role", e.to_string()))?;

        member
            .add_role(http.as_ref(), role_id)
            .await
            .map_err(|e| DiscordToolError::api_error("assign_role", e.to_string()))?;

        tracing::info!(
            "Role {} assigned to user {} in guild {} by user {}",
            args.role_id,
            args.user_id,
            args.guild_id,
            args.requesting_user_id
        );

        Ok(format!(
            "Successfully assigned role {} to user {}",
            args.role_id, args.user_id
        ))
    }
}

// ── RemoveRole ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RemoveRoleArgs {
    guild_id: u64,
    user_id: u64,
    role_id: u64,
    /// この操作を指示したユーザーのID（メタデータから取得）
    requesting_user_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct RemoveRole {
    #[serde(skip)]
    http: Option<Arc<Http>>,
    #[serde(skip)]
    cache: Option<Arc<Cache>>,
}

impl RemoveRole {
    pub fn new(http: Arc<Http>, cache: Arc<Cache>) -> Self {
        Self {
            http: Some(http),
            cache: Some(cache),
        }
    }
}

impl Tool for RemoveRole {
    const NAME: &'static str = "remove_role";
    type Error = DiscordToolError;
    type Args = RemoveRoleArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "remove_role".to_string(),
            description:
                "Remove a role from a user. The requesting user must have MANAGE_ROLES permission."
                    .to_string(),
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
                    },
                    "role_id": {
                        "type": "integer",
                        "description": "The Discord role ID to remove"
                    },
                    "requesting_user_id": {
                        "type": "integer",
                        "description": "The user ID of the person who requested this action (from metadata)"
                    }
                },
                "required": ["guild_id", "user_id", "role_id", "requesting_user_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("remove_role", "HTTP client not available"))?;
        let cache = self
            .cache
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("remove_role", "Cache not available"))?;

        let guild_id = GuildId::new(args.guild_id);
        let user_id = UserId::new(args.user_id);
        let role_id = RoleId::new(args.role_id);
        let requesting_user_id = UserId::new(args.requesting_user_id);

        // MANAGE_ROLES 権限チェック
        require_user_permission(
            cache,
            http,
            guild_id,
            requesting_user_id,
            Permissions::MANAGE_ROLES,
            "remove_role",
        )
        .await?;

        let member = guild_id
            .member(http.as_ref(), user_id)
            .await
            .map_err(|e| DiscordToolError::not_found("remove_role", e.to_string()))?;

        member
            .remove_role(http.as_ref(), role_id)
            .await
            .map_err(|e| DiscordToolError::api_error("remove_role", e.to_string()))?;

        tracing::info!(
            "Role {} removed from user {} in guild {} by user {}",
            args.role_id,
            args.user_id,
            args.guild_id,
            args.requesting_user_id
        );

        Ok(format!(
            "Successfully removed role {} from user {}",
            args.role_id, args.user_id
        ))
    }
}
