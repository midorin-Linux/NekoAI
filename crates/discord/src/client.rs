use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use nekoai_agent::runtime::AgentRuntime;
use nekoai_tools::discord::{
    channel::{
        CreateDiscordChannel, DeleteDiscordChannel, GetDiscordChannelInfo,
        GetDiscordChannelList, ModifyDiscordChannel,
    },
    emoji::{CreateDiscordEmoji, DeleteDiscordEmoji, GetDiscordEmojiList, GetDiscordStickerList},
    guild::{GetDiscordAuditLog, GetDiscordGuildInfo, GetDiscordGuildList, ModifyDiscordGuild},
    invite::{CreateDiscordInvite, DeleteDiscordInvite, GetDiscordInviteList},
    member::{
        BanDiscordMember, BulkBanDiscordMembers, GetDiscordMemberInfo, GetDiscordMemberList,
        KickDiscordMember, ModifyDiscordMember, TimeoutDiscordMember, UnbanDiscordMember,
    },
    message::{
        AddDiscordReaction, BulkDeleteDiscordMessages, DeleteDiscordMessage,
        EditDiscordMessage, GetDiscordMessage, GetDiscordMessageHistory,
        PinDiscordMessage, RemoveDiscordReaction, SendDiscordMessage,
        UnpinDiscordMessage,
    },
    role::{
        AddDiscordRoleToMember, CreateDiscordRole, DeleteDiscordRole, GetDiscordRoleList,
        ModifyDiscordRole, RemoveDiscordRoleFromMember,
    },
    schedule::{
        CreateDiscordScheduledEvent, DeleteDiscordScheduledEvent,
        GetDiscordScheduledEvents, ModifyDiscordScheduledEvent,
    },
    thread::{AddDiscordThreadMember, CreateDiscordThread, DeleteDiscordThread, GetDiscordThreadList},
    voice::{
        DeafenDiscordMember, DisconnectDiscordMemberVoice, MoveDiscordMemberVoice,
        MuteDiscordMember,
    },
};
use serenity::http::Http;
use serenity::prelude::*;
use std::sync::Arc;
use tracing::info;

use crate::handler::Handler;

pub struct DiscordClient {
    discord_client: Client,
}

impl DiscordClient {
    pub async fn new(
        discord_token: String,
        guild_id: u64,
        agent_runtime: AgentRuntime,
    ) -> Result<Self> {
        info!(guild_id, "creating discord client");
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("    {spinner} Starting discord client...")?,
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(120));

        let intents = GatewayIntents::GUILDS
            | GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        let runtime_for_tools = agent_runtime.clone();

        let command_framework =
            crate::command_router::command_framework(guild_id, agent_runtime.clone()).await;
        info!("discord command framework initialized");

        let discord_client = Client::builder(&discord_token, intents)
            .event_handler(Handler {
                agent_runtime,
                spinner,
            })
            .framework(command_framework)
            .await?;

        let http = Arc::new(Http::new(&discord_token));
        macro_rules! register_tools {
            ($($tool:expr),* $(,)?) => {{
                $(runtime_for_tools.add_tool($tool).await;)*
            }};
        }

        register_tools!(
            SendDiscordMessage::new(http.clone()),
            EditDiscordMessage::new(http.clone()),
            DeleteDiscordMessage::new(http.clone()),
            GetDiscordMessage::new(http.clone()),
            BulkDeleteDiscordMessages::new(http.clone()),
            GetDiscordMessageHistory::new(http.clone()),
            PinDiscordMessage::new(http.clone()),
            UnpinDiscordMessage::new(http.clone()),
            AddDiscordReaction::new(http.clone()),
            RemoveDiscordReaction::new(http.clone()),
        );

        register_tools!(
            CreateDiscordChannel::new(http.clone()),
            DeleteDiscordChannel::new(http.clone()),
            ModifyDiscordChannel::new(http.clone()),
            GetDiscordChannelInfo::new(http.clone()),
            GetDiscordChannelList::new(http.clone()),
        );

        register_tools!(
            GetDiscordGuildInfo::new(http.clone()),
            GetDiscordGuildList::new(http.clone()),
            ModifyDiscordGuild::new(http.clone()),
            GetDiscordAuditLog::new(http.clone()),
        );

        register_tools!(
            GetDiscordRoleList::new(http.clone()),
            CreateDiscordRole::new(http.clone()),
            DeleteDiscordRole::new(http.clone()),
            ModifyDiscordRole::new(http.clone()),
            AddDiscordRoleToMember::new(http.clone()),
            RemoveDiscordRoleFromMember::new(http.clone()),
        );

        register_tools!(
            GetDiscordMemberList::new(http.clone()),
            GetDiscordMemberInfo::new(http.clone()),
            KickDiscordMember::new(http.clone()),
            BanDiscordMember::new(http.clone()),
            UnbanDiscordMember::new(http.clone()),
            BulkBanDiscordMembers::new(http.clone()),
            ModifyDiscordMember::new(http.clone()),
            TimeoutDiscordMember::new(http.clone()),
        );

        register_tools!(
            CreateDiscordThread::new(http.clone()),
            DeleteDiscordThread::new(http.clone()),
            GetDiscordThreadList::new(http.clone()),
            AddDiscordThreadMember::new(http.clone()),
        );

        register_tools!(
            MoveDiscordMemberVoice::new(http.clone()),
            DisconnectDiscordMemberVoice::new(http.clone()),
            MuteDiscordMember::new(http.clone()),
            DeafenDiscordMember::new(http.clone()),
        );

        register_tools!(
            GetDiscordInviteList::new(http.clone()),
            CreateDiscordInvite::new(http.clone()),
            DeleteDiscordInvite::new(http.clone()),
        );

        register_tools!(
            GetDiscordEmojiList::new(http.clone()),
            CreateDiscordEmoji::new(http.clone()),
            DeleteDiscordEmoji::new(http.clone()),
            GetDiscordStickerList::new(http.clone()),
        );

        register_tools!(
            GetDiscordScheduledEvents::new(http.clone()),
            CreateDiscordScheduledEvent::new(http.clone()),
            ModifyDiscordScheduledEvent::new(http.clone()),
            DeleteDiscordScheduledEvent::new(http.clone()),
        );

        info!("discord client created");

        Ok(Self { discord_client })
    }

    pub async fn run(mut self) -> Result<()> {
        info!("starting discord event loop");
        self.discord_client
            .start()
            .await
            .context("Failed to start Discord client")?;

        info!("discord event loop finished");

        Ok(())
    }
}
