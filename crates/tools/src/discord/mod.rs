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

pub use channel::{
    ArchiveChannel, CreateChannelTool, ListChannels, SetChannelPermissions, UpdateChannel,
};
pub use emoji::{AddEmoji, DeleteEmoji, GetReactionStats, ListEmojis};
pub use error::DiscordToolError;
pub use guild::{GetAuditLog, GetGuildInfo, ManageBans, UpdateGuildSettings};
pub use invite::{CreateInviteTool, ListInvites, RevokeInvite};
pub use member::{
    GetMemberActivity, KickMember, SearchMembers, TimeoutMember, UpdateMemberNickname,
};
pub use message::{
    AddReaction, BulkDeleteMessages, PinMessage, SearchMessages, SendMessageTool,
    SendWebhookMessage,
};
pub use role::{AssignRoles, ListRoleMembers, ListRoles, ReorderRoles, UpsertRole};
pub use schedule::{
    CreateScheduledEventTool, GetEventSubscribers, ListEvents, UpdateOrCancelEvent,
};
pub use thread::{ArchiveOrLockThread, CreateThreadTool, ListThreads, ManageThreadMembers};
pub use voice::{GetVoiceStates, ManageStageTopic, MoveMemberToVoice, SetVoiceMuteDeafen};
