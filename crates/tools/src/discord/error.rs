//! Error types for Discord tools.

use thiserror::Error;

/// Errors that can occur when executing Discord tools.
#[derive(Debug, Error)]
pub enum DiscordToolError {
    /// An error from the Discord API.
    #[error("Discord API error: {0}")]
    Serenity(#[from] serenity::Error),
}
