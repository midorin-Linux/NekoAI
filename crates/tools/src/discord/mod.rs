//! Discord tools for NekoAI.
//!
//! Tools in this module interact with Discord's API via the serenity client.
//! Each tool implements the [`rig::tool::Tool`] trait, allowing it to be used
//! by Rig agents.

pub mod error;
pub(crate) mod helpers;
pub mod channel;
pub mod guild;
pub mod message;
pub mod role;
pub mod member;
pub mod thread;
pub mod voice;
pub mod invite;
pub mod emoji;
pub mod schedule;

pub use error::DiscordToolError;
pub use message::{SendDiscordMessage, SendMessageArgs, SendMessageOutput};
