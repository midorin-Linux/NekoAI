use anyhow::Result;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
pub struct OperationArgs {
    content: String,
    target_channel_id: u64,
}

#[derive(Debug, thiserror::Error)]
#[error("Discord message send error")]
pub struct DiscordMessageSendError;

#[derive(Deserialize, Serialize)]
pub struct Test;
impl Tool for Test {
    const NAME: &'static str = "send_message";
    type Error = DiscordMessageSendError;
    type Args = OperationArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "send_message".to_string(),
            description: "Send message to target channel".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "Message content"
                    },
                    "target_channel_id": {
                        "type": "integer",
                        "description": "Target channel ID"
                    },
                },
                "required": ["content", "target_channel_id"],
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let result = "Succeccfly send message".to_string();
        Ok(result)
    }
}
