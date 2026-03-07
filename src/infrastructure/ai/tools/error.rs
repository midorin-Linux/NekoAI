use std::fmt;

/// Discord ツール共通のエラー型
#[derive(Debug)]
pub struct DiscordToolError {
    pub tool_name: String,
    pub message: String,
}

impl DiscordToolError {
    pub fn new(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for DiscordToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.tool_name, self.message)
    }
}

impl std::error::Error for DiscordToolError {}
