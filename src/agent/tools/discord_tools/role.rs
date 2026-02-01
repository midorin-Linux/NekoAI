use crate::agent::tools::tools::build_tool;
use crate::agent::tools::discord::{
    err, get_bool, get_guild_id_default, get_string, get_u64, get_user_id, ok, parse_colour, to_value
};

use anyhow::Result;
use async_openai::types::chat::ChatCompletionTools;
use serde_json::{json, Value};
use serenity::all::{Context, EditRole, RoleId};

pub fn definitions() -> Result<Vec<ChatCompletionTools>> {
    let mut tools = Vec::new();

    tools.push(build_tool(
        "get_role_list",
        "List guild roles.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." }
            },
            "required": ["guild_id"]
        }),
    )?);

    tools.push(build_tool(
        "create_role",
        "Create a role in the guild.",
        json!({
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
    )?);

    tools.push(build_tool(
        "delete_role",
        "Delete a role from the guild.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "role_id": { "type": "integer", "description": "Role id." }
            },
            "required": ["guild_id", "role_id"]
        }),
    )?);

    tools.push(build_tool(
        "modify_role",
        "Modify a role in the guild.",
        json!({
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
    )?);

    tools.push(build_tool(
        "add_role_to_member",
        "Add a role to a member.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "user_id": { "type": "integer", "description": "User id." },
                "role_id": { "type": "integer", "description": "Role id." }
            },
            "required": ["guild_id", "user_id", "role_id"]
        }),
    )?);

    tools.push(build_tool(
        "remove_role_from_member",
        "Remove a role from a member.",
        json!({
            "type": "object",
            "properties": {
                "guild_id": { "type": "integer", "description": "Guild id." },
                "user_id": { "type": "integer", "description": "User id." },
                "role_id": { "type": "integer", "description": "Role id." }
            },
            "required": ["guild_id", "user_id", "role_id"]
        }),
    )?);

    Ok(tools)
}

async fn get_role_list(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };

    match guild_id.roles(&ctx.http).await {
        Ok(roles) => {
            let role_list = roles.values().cloned().collect::<Vec<_>>();
            ok(to_value(&role_list))
        }
        Err(error) => err(format!("Failed to fetch role list: {error}")),
    }
}

async fn create_role(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };

    let name = get_string(args, "name").unwrap_or_else(|| "New Role".to_string());
    let permissions = get_u64(args, "permissions").unwrap_or(0);
    let color = args.get("color").and_then(parse_colour);
    let hoist = get_bool(args, "hoist").unwrap_or(false);
    let mentionable = get_bool(args, "mentionable").unwrap_or(false);

    let builder = EditRole::new()
        .name(name)
        .permissions(serenity::all::Permissions::from_bits_truncate(permissions))
        .hoist(hoist)
        .mentionable(mentionable);
    
    let builder = if let Some(color) = color {
        builder.colour(color)
    } else {
        builder
    };

    match ctx.http.create_role(guild_id, &builder, None).await {
        Ok(role) => ok(to_value(&role)),
        Err(error) => err(format!("Failed to create role: {error}")),
    }
}

async fn delete_role(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(role_id) = get_u64(args, "role_id").map(RoleId::new) else {
        return err("role_id is required");
    };

    match guild_id.delete_role(&ctx.http, role_id).await {
        Ok(()) => ok(json!({ "deleted": true })),
        Err(error) => err(format!("Failed to delete role: {error}")),
    }
}

async fn modify_role(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(role_id) = get_u64(args, "role_id").map(RoleId::new) else {
        return err("role_id is required");
    };

    let mut builder = EditRole::new();
    let mut changed = false;

    if let Some(name) = get_string(args, "name") {
        builder = builder.name(name);
        changed = true;
    }
    if let Some(permissions) = get_u64(args, "permissions") {
        builder = builder.permissions(serenity::all::Permissions::from_bits_truncate(permissions));
        changed = true;
    }
    if let Some(color) = args.get("color").and_then(parse_colour) {
        builder = builder.colour(color);
        changed = true;
    }
    if let Some(hoist) = get_bool(args, "hoist") {
        builder = builder.hoist(hoist);
        changed = true;
    }
    if let Some(mentionable) = get_bool(args, "mentionable") {
        builder = builder.mentionable(mentionable);
        changed = true;
    }

    if !changed {
        return err("No role fields provided to modify");
    }

    match guild_id.edit_role(ctx, role_id, builder).await {
        Ok(role) => ok(to_value(&role)),
        Err(error) => err(format!("Failed to modify role: {error}")),
    }
}

async fn add_role_to_member(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(user_id) = get_user_id(args, "user_id") else {
        return err("user_id is required");
    };
    let Some(role_id) = get_u64(args, "role_id").map(RoleId::new) else {
        return err("role_id is required");
    };

    match guild_id.member(ctx, user_id).await {
        Ok(member) => {
            match member.add_role(ctx, role_id).await {
                Ok(()) => ok(json!({ "added": true })),
                Err(error) => err(format!("Failed to add role to member: {error}")),
            }
        }
        Err(error) => err(format!("Failed to fetch member: {error}")),
    }
}

async fn remove_role_from_member(ctx: &Context, args: &Value) -> String {
    let Some(guild_id) = get_guild_id_default(args) else {
        return err("guild_id is required");
    };
    let Some(user_id) = get_user_id(args, "user_id") else {
        return err("user_id is required");
    };
    let Some(role_id) = get_u64(args, "role_id").map(RoleId::new) else {
        return err("role_id is required");
    };

    match guild_id.member(ctx, user_id).await {
        Ok(member) => {
            match member.remove_role(ctx, role_id).await {
                Ok(()) => ok(json!({ "removed": true })),
                Err(error) => err(format!("Failed to remove role from member: {error}")),
            }
        }
        Err(error) => err(format!("Failed to fetch member: {error}")),
    }
}

pub async fn execute(ctx: &Context, name: &str, args: &Value) -> Option<String> {
    match name {
        "get_role_list" => Some(get_role_list(ctx, args).await),
        "create_role" => Some(create_role(ctx, args).await),
        "delete_role" => Some(delete_role(ctx, args).await),
        "modify_role" => Some(modify_role(ctx, args).await),
        "add_role_to_member" => Some(add_role_to_member(ctx, args).await),
        "remove_role_from_member" => Some(remove_role_from_member(ctx, args).await),
        _ => None,
    }
}
