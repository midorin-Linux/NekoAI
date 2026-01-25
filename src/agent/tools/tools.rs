use anyhow::Result;
use async_openai::types::chat::{ChatCompletionTool, ChatCompletionTools, FunctionObjectArgs};
use serde_json::{json, Value};
use serenity::all::Context;

use crate::agent::tools::get_channel_id_list_from_guild_id;

pub fn tool_definitions() -> Result<Vec<ChatCompletionTools>> {
    let get_channel_id_list_from_guild_id_tool = ChatCompletionTools::Function(ChatCompletionTool {
        function: FunctionObjectArgs::default()
            .name("get_channel_id_list_from_guild_id")
            .description("List channel id from guild id. Columns are output as `<channel_name>: <channel_id> (<channel_type>)`.")
            .parameters(json!({
                "type": "object",
                "properties": {
                    "guild_id": {
                        "type": "integer",
                        "description": "Guild id."
                    },
                },
                "required": ["guild_id"]
            }))
            .build()?,
    });

    Ok(vec![get_channel_id_list_from_guild_id_tool])
}

pub async fn execute_tool_call(ctx: &Context, name: &str, arguments: &str) -> String {
    let args: Value = serde_json::from_str(arguments).unwrap_or_else(|_| json!({}));
    tracing::info!("Colling tool: {:#?} with args: {:?}", name, args);

    match name {
        "get_channel_id_list_from_guild_id" => {
            get_channel_id_list_from_guild_id::execute(ctx, &args).await
        }
        _ => json!({ "error": format!("unknown tool: {}", name) }).to_string(),
    }
}
