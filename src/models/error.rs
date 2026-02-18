use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("AI error: {0}")]
    AIError(String),

    #[error("Discord error: {0}")]
    DiscordError(String),

    #[error("Conversation not found: channel {0}")]
    ConversationNotFound(u64),

    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },

    #[error(transparent)]
    InternalError(#[from] anyhow::Error),
}
