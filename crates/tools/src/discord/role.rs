use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::{Value, json};
use serenity::{
    all::{EditRole, Permissions, RoleId},
    http::Http,
};

use crate::discord::{
    error::DiscordToolError,
    helpers::{
        err, get_bool, get_guild_id_default, get_string, get_u64, get_user_id, ok, parse_colour,
        retry_discord, to_value,
    },
};

pub struct GetDiscordRoleList {
    http: Arc<Http>,
}

pub struct CreateDiscordRole {
    http: Arc<Http>,
}

pub struct DeleteDiscordRole {
    http: Arc<Http>,
}

pub struct ModifyDiscordRole {
    http: Arc<Http>,
}

pub struct AddDiscordRoleToMember {
    http: Arc<Http>,
}

pub struct RemoveDiscordRoleFromMember {
    http: Arc<Http>,
}

impl GetDiscordRoleList {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl CreateDiscordRole {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl DeleteDiscordRole {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl ModifyDiscordRole {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl AddDiscordRoleToMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl RemoveDiscordRoleFromMember {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for GetDiscordRoleList {
    const NAME: &'static str = "get_discord_role_list";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "List guild roles.".to_string(),
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
            async move { guild_id.roles(&http).await }
        })
        .await
        {
            Ok(roles) => {
                let role_list = roles.values().cloned().collect::<Vec<_>>();
                Ok(ok(to_value(&role_list)))
            }
            Err(error) => Ok(err(format!("Failed to fetch role list: {error}"))),
        }
    }
}

impl Tool for CreateDiscordRole {
    const NAME: &'static str = "create_discord_role";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Create a role in the guild.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "name": { "type": "string", "description": "Role name." },
                    "permissions": { "type": "integer", "description": "Permissions bitset." },
                    "color": { "type": "string", "description": "Role color hex (e.g. #ff0000)." },
                    "hoist": { "type": "boolean", "description": "Display role separately." },
                    "mentionable": { "type": "boolean", "description": "Allow role mentions." }
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

        let name = get_string(&args, "name").unwrap_or_else(|| "New Role".to_string());
        let permissions = get_u64(&args, "permissions").unwrap_or(0);
        let color = args.get("color").and_then(parse_colour);
        let hoist = get_bool(&args, "hoist").unwrap_or(false);
        let mentionable = get_bool(&args, "mentionable").unwrap_or(false);

        let builder = EditRole::new()
            .name(name)
            .permissions(Permissions::from_bits_truncate(permissions))
            .hoist(hoist)
            .mentionable(mentionable);
        let builder = if let Some(color) = color {
            builder.colour(color)
        } else {
            builder
        };

        let http = self.http.clone();
        let builder = builder.clone();
        match retry_discord(|| {
            let http = http.clone();
            let builder = builder.clone();
            async move { guild_id.create_role(&http, builder).await }
        })
        .await
        {
            Ok(role) => Ok(ok(to_value(&role))),
            Err(error) => Ok(err(format!("Failed to create role: {error}"))),
        }
    }
}

impl Tool for DeleteDiscordRole {
    const NAME: &'static str = "delete_discord_role";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Delete a role from the guild.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "role_id": { "type": "integer", "description": "Role id." }
                },
                "required": ["guild_id", "role_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);
        let Some(role_id) = get_u64(&args, "role_id").map(RoleId::new) else {
            return Ok(err("role_id is required"));
        };

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.delete_role(&http, role_id).await }
        })
        .await
        {
            Ok(()) => Ok(ok(json!({ "deleted": true }))),
            Err(error) => Ok(err(format!("Failed to delete role: {error}"))),
        }
    }
}

impl Tool for ModifyDiscordRole {
    const NAME: &'static str = "modify_discord_role";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Modify a role in the guild.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "role_id": { "type": "integer", "description": "Role id." },
                    "name": { "type": "string", "description": "Role name." },
                    "permissions": { "type": "integer", "description": "Permissions bitset." },
                    "color": { "type": "string", "description": "Role color hex (e.g. #ff0000)." },
                    "hoist": { "type": "boolean", "description": "Display role separately." },
                    "mentionable": { "type": "boolean", "description": "Allow role mentions." }
                },
                "required": ["guild_id", "role_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let Some(guild_id) = get_guild_id_default(&args) else {
            return Ok(err("guild_id is required"));
        };
        crate::admin_guard_guild!(&self.http, guild_id);
        let Some(role_id) = get_u64(&args, "role_id").map(RoleId::new) else {
            return Ok(err("role_id is required"));
        };

        let mut builder = EditRole::new();
        let mut changed = false;

        if let Some(name) = get_string(&args, "name") {
            builder = builder.name(name);
            changed = true;
        }
        if let Some(permissions) = get_u64(&args, "permissions") {
            builder = builder.permissions(Permissions::from_bits_truncate(permissions));
            changed = true;
        }
        if let Some(color) = args.get("color").and_then(parse_colour) {
            builder = builder.colour(color);
            changed = true;
        }
        if let Some(hoist) = get_bool(&args, "hoist") {
            builder = builder.hoist(hoist);
            changed = true;
        }
        if let Some(mentionable) = get_bool(&args, "mentionable") {
            builder = builder.mentionable(mentionable);
            changed = true;
        }

        if !changed {
            return Ok(err("No role fields provided to modify"));
        }

        let http = self.http.clone();
        let builder = builder.clone();
        match retry_discord(|| {
            let http = http.clone();
            let builder = builder.clone();
            async move { guild_id.edit_role(&http, role_id, builder).await }
        })
        .await
        {
            Ok(role) => Ok(ok(to_value(&role))),
            Err(error) => Ok(err(format!("Failed to modify role: {error}"))),
        }
    }
}

impl Tool for AddDiscordRoleToMember {
    const NAME: &'static str = "add_discord_role_to_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Add a role to a member.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." },
                    "role_id": { "type": "integer", "description": "Role id." }
                },
                "required": ["guild_id", "user_id", "role_id"]
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
        let Some(role_id) = get_u64(&args, "role_id").map(RoleId::new) else {
            return Ok(err("role_id is required"));
        };

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.member(&http, user_id).await }
        })
        .await
        {
            Ok(member) => {
                let http = self.http.clone();
                let member = member.clone();
                match retry_discord(|| {
                    let http = http.clone();
                    let member = member.clone();
                    async move { member.add_role(&http, role_id).await }
                })
                .await
                {
                    Ok(()) => Ok(ok(json!({ "added": true }))),
                    Err(error) => Ok(err(format!("Failed to add role to member: {error}"))),
                }
            }
            Err(error) => Ok(err(format!("Failed to fetch member: {error}"))),
        }
    }
}

impl Tool for RemoveDiscordRoleFromMember {
    const NAME: &'static str = "remove_discord_role_from_member";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Remove a role from a member.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "guild_id": { "type": "integer", "description": "Guild id." },
                    "user_id": { "type": "integer", "description": "User id." },
                    "role_id": { "type": "integer", "description": "Role id." }
                },
                "required": ["guild_id", "user_id", "role_id"]
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
        let Some(role_id) = get_u64(&args, "role_id").map(RoleId::new) else {
            return Ok(err("role_id is required"));
        };

        let http = self.http.clone();
        match retry_discord(|| {
            let http = http.clone();
            async move { guild_id.member(&http, user_id).await }
        })
        .await
        {
            Ok(member) => {
                let http = self.http.clone();
                let member = member.clone();
                match retry_discord(|| {
                    let http = http.clone();
                    let member = member.clone();
                    async move { member.remove_role(&http, role_id).await }
                })
                .await
                {
                    Ok(()) => Ok(ok(json!({ "removed": true }))),
                    Err(error) => Ok(err(format!("Failed to remove role from member: {error}"))),
                }
            }
            Err(error) => Ok(err(format!("Failed to fetch member: {error}"))),
        }
    }
}
