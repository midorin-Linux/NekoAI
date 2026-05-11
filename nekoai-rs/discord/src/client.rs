use std::sync::Arc;

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use nekoai_agent::runtime::AgentRuntime;
use nekoai_tools::discord::{
    channel::{
        ArchiveChannel, CreateChannelTool, ListChannels, SetChannelPermissions, UpdateChannel,
    },
    emoji::{AddEmoji, DeleteEmoji, GetReactionStats, ListEmojis},
    guild::{GetAuditLog, GetGuildInfo, ManageBans, UpdateGuildSettings},
    invite::{CreateInviteTool, ListInvites, RevokeInvite},
    member::{
        GetMemberActivity, InvestigateMember, KickMember, ManageMemberRoles, ModerateMember,
        SearchMembers, TimeoutMember, UpdateMemberNickname,
    },
    message::{
        AddReaction, BulkDeleteMessages, CreatePoll, FetchReadableChatHistory, PinMessage,
        SearchChannelMessages, SearchMessages, SendAnnouncementWithPin, SendMessageTool,
        SendWebhookMessage,
    },
    role::{
        AssignRoleByName, AssignRoleToMultipleMembers, AssignRoles, ClearRoleFromAllMembers,
        CreateAndAssignRole, DuplicateRole, GetMembersWithRole, ListRoleMembers, ListRoles,
        ReorderRoles, RevokeRoleByName, UpsertRole,
    },
    schedule::{CreateScheduledEventTool, GetEventSubscribers, ListEvents, UpdateOrCancelEvent},
    thread::{ArchiveOrLockThread, CreateThreadTool, ListThreads, ManageThreadMembers},
    voice::{GetVoiceStates, ManageStageTopic, MoveMemberToVoice, SetVoiceMuteDeafen},
};
use serenity::{http::Http, prelude::*};
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
            ListChannels::new(http.clone()),
            CreateChannelTool::new(http.clone()),
            UpdateChannel::new(http.clone()),
            ArchiveChannel::new(http.clone()),
            SetChannelPermissions::new(http.clone()),
            ListEmojis::new(http.clone()),
            AddEmoji::new(http.clone()),
            DeleteEmoji::new(http.clone()),
            GetReactionStats::new(http.clone()),
            GetGuildInfo::new(http.clone()),
            UpdateGuildSettings::new(http.clone()),
            GetAuditLog::new(http.clone()),
            ManageBans::new(http.clone()),
            CreateInviteTool::new(http.clone()),
            ListInvites::new(http.clone()),
            RevokeInvite::new(http.clone()),
            SearchMembers::new(http.clone()),
            UpdateMemberNickname::new(http.clone()),
            TimeoutMember::new(http.clone()),
            KickMember::new(http.clone()),
            GetMemberActivity::new(http.clone()),
            ManageMemberRoles::new(http.clone()),
            InvestigateMember::new(http.clone()),
            ModerateMember::new(http.clone()),
            SendMessageTool::new(http.clone()),
            SearchMessages::new(http.clone()),
            BulkDeleteMessages::new(http.clone()),
            PinMessage::new(http.clone()),
            AddReaction::new(http.clone()),
            SendWebhookMessage::new(http.clone()),
            FetchReadableChatHistory::new(http.clone()),
            SearchChannelMessages::new(http.clone()),
            CreatePoll::new(http.clone()),
            SendAnnouncementWithPin::new(http.clone()),
            ListRoles::new(http.clone()),
            UpsertRole::new(http.clone()),
            AssignRoles::new(http.clone()),
            ReorderRoles::new(http.clone()),
            ListRoleMembers::new(http.clone()),
            AssignRoleByName::new(http.clone()),
            RevokeRoleByName::new(http.clone()),
            GetMembersWithRole::new(http.clone()),
            ClearRoleFromAllMembers::new(http.clone()),
            AssignRoleToMultipleMembers::new(http.clone()),
            CreateAndAssignRole::new(http.clone()),
            DuplicateRole::new(http.clone()),
            CreateScheduledEventTool::new(http.clone()),
            ListEvents::new(http.clone()),
            UpdateOrCancelEvent::new(http.clone()),
            GetEventSubscribers::new(http.clone()),
            CreateThreadTool::new(http.clone()),
            ListThreads::new(http.clone()),
            ArchiveOrLockThread::new(http.clone()),
            ManageThreadMembers::new(http.clone()),
            GetVoiceStates::new(http.clone()),
            MoveMemberToVoice::new(http.clone()),
            SetVoiceMuteDeafen::new(http.clone()),
            ManageStageTopic::new(http.clone()),
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
