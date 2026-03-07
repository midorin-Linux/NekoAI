use std::sync::Arc;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::all::{ChannelId, Http};

use super::error::DiscordToolError;

#[derive(Deserialize)]
pub struct OperationArgs {
    content: String,
    channel_id: u64,
}

#[derive(Serialize, Deserialize)]
pub struct SendMessage {
    #[serde(skip)]
    http: Option<Arc<Http>>,
}

impl SendMessage {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http: Some(http) }
    }
}

impl Tool for SendMessage {
    const NAME: &'static str = "send_message";
    type Error = DiscordToolError;
    type Args = OperationArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "send_message".to_string(),
            description: "Send a message to a specified Discord channel by its channel ID."
                .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "The message content to send"
                    },
                    "channel_id": {
                        "type": "integer",
                        "description": "The Discord channel ID to send the message to"
                    }
                },
                "required": ["content", "channel_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self.http.as_ref().ok_or_else(|| {
            DiscordToolError::new("send_message", "Discord HTTP client not available")
        })?;

        let channel_id = ChannelId::new(args.channel_id);

        channel_id
            .say(http.as_ref(), &args.content)
            .await
            .map_err(|e| DiscordToolError::new("send_message", e.to_string()))?;

        Ok(format!(
            "Successfully sent message to channel {}",
            args.channel_id
        ))
    }
}
