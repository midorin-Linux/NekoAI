//! Discord tools for NekoAI.
//!
//! Tools in this module interact with Discord's API via the serenity client.
//! Each tool implements the [`rig::tool::Tool`] trait, allowing it to be used
//! by Rig agents.

pub mod channel;
pub mod emoji;
pub mod error;
pub mod guild;
pub(crate) mod helpers;
pub mod invite;
pub mod member;
pub mod message;
pub mod permission;
pub mod role;
pub mod schedule;
pub mod thread;
pub mod voice;

pub use error::DiscordToolError;
pub use message::{SendDiscordMessage, SendMessageArgs, SendMessageOutput};
