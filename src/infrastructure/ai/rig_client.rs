use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use rig::{
    completion::{Chat, Message, request::PromptError},
    embeddings::EmbeddingModel,
    prelude::*,
    providers,
};
use serenity::all::{Cache, Http};

use crate::{
    application::traits::ai_client::AIClient,
    infrastructure::ai::tools::*,
    models::memory::{ChatMessage, ChatRole},
    shared::config::{Embedding, NLP},
};

pub struct RigClient {
    nlp_client: rig::agent::Agent<providers::openai::responses_api::ResponsesCompletionModel>,
    embed_client: rig::providers::openai::EmbeddingModel,
}

impl RigClient {
    pub async fn new(
        nlp_api_key: String,
        embed_api_key: String,
        nlp: NLP,
        embedding: Embedding,
        discord_http: Arc<Http>,
        discord_cache: Arc<Cache>,
    ) -> Result<Self> {
        let system_instruction =
            std::fs::read_to_string("INSTRUCTION.md").context("Failed to read INSTRUCTION.md")?;

        let openai_comp_nlp_client = providers::openai::Client::builder()
            .api_key(nlp_api_key)
            .base_url(nlp.api_url)
            .build()
            .context("Failed to build openai nlp client")?;

        // リマインダーストアを作成
        let reminder_store = reminder::new_reminder_store();

        let nlp_client = openai_comp_nlp_client
            .agent(nlp.model_name)
            .preamble(system_instruction.as_str())
            // ── 既存ツール ──
            .tool(send_message::SendMessage::new(discord_http.clone()))
            // ── チャンネル管理 ──
            .tool(channel::GetChannelInfo::new(discord_http.clone()))
            .tool(channel::ListChannels::new(
                discord_http.clone(),
                discord_cache.clone(),
            ))
            .tool(channel::CreateChannel::new(
                discord_http.clone(),
                discord_cache.clone(),
            ))
            .tool(channel::EditChannelTool::new(
                discord_http.clone(),
                discord_cache.clone(),
            ))
            .tool(channel::DeleteChannel::new(
                discord_http.clone(),
                discord_cache.clone(),
            ))
            // ── ロール管理 ──
            .tool(role::ListRoles::new(discord_cache.clone()))
            .tool(role::GetRoleInfo::new(discord_cache.clone()))
            .tool(role::AssignRole::new(
                discord_http.clone(),
                discord_cache.clone(),
            ))
            .tool(role::RemoveRole::new(
                discord_http.clone(),
                discord_cache.clone(),
            ))
            // ── メンバー情報 ──
            .tool(member::GetMemberInfo::new(discord_http.clone()))
            .tool(member::SearchMembers::new(discord_http.clone()))
            // ── メッセージ管理 ──
            .tool(message::GetMessage::new(discord_http.clone()))
            .tool(message::EditMessageTool::new(discord_http.clone()))
            .tool(message::DeleteMessage::new(
                discord_http.clone(),
                discord_cache.clone(),
            ))
            .tool(message::PinMessage::new(discord_http.clone()))
            .tool(message::AddReaction::new(discord_http.clone()))
            .tool(message::SendReply::new(discord_http.clone()))
            // ── ボイスチャンネル ──
            .tool(voice::GetVoiceChannelInfo::new(discord_http.clone()))
            .tool(voice::ListVoiceMembers::new(discord_cache.clone()))
            // ── Embed 送信 ──
            .tool(embed::SendEmbed::new(discord_http.clone()))
            // ── サーバー情報 ──
            .tool(server::GetServerInfo::new(discord_cache.clone()))
            .tool(server::GetServerStats::new(discord_cache.clone()))
            // ── リマインダー ──
            .tool(reminder::SetReminder::new(
                discord_http.clone(),
                reminder_store.clone(),
            ))
            .tool(reminder::ListReminders::new(reminder_store.clone()))
            .tool(reminder::CancelReminder::new(reminder_store.clone()))
            // ── モデレーション ──
            .tool(moderation::KickMember::new(
                discord_http.clone(),
                discord_cache.clone(),
            ))
            .tool(moderation::BanMember::new(
                discord_http.clone(),
                discord_cache.clone(),
            ))
            .tool(moderation::TimeoutMember::new(
                discord_http.clone(),
                discord_cache.clone(),
            ))
            .tool(moderation::WarnMember::new(
                discord_http.clone(),
                discord_cache.clone(),
            ))
            .default_max_turns(nlp.max_agent_turns)
            .build();

        let openai_comp_embed_client = providers::openai::Client::builder()
            .api_key(embed_api_key)
            .base_url(embedding.api_url)
            .build()
            .context("Failed to build openai embed client")?;

        let embed_client = openai_comp_embed_client.embedding_model(embedding.model_name);

        Ok(Self {
            nlp_client,
            embed_client,
        })
    }
}

fn to_rig_message(msg: ChatMessage) -> Message {
    match msg.role {
        ChatRole::User => Message::user(msg.content),
        ChatRole::Assistant => Message::assistant(msg.content),
    }
}

#[async_trait]
impl AIClient for RigClient {
    async fn generate(
        &self,
        prompt: ChatMessage,
        chat_history: Vec<ChatMessage>,
    ) -> Result<String> {
        let rig_prompt = to_rig_message(prompt);
        let rig_history: Vec<Message> = chat_history.into_iter().map(to_rig_message).collect();

        self.nlp_client
            .chat(rig_prompt, rig_history)
            .await
            .map_err(|e: PromptError| anyhow!(e.to_string()))
    }

    async fn embed(&self, text: String) -> Result<Vec<f32>> {
        let embeddings = self
            .embed_client
            .embed_texts(vec![text])
            .await
            .map_err(|e| anyhow!(e.to_string()))?;

        embeddings
            .into_iter()
            .next()
            .map(|e| e.vec.into_iter().map(|v| v as f32).collect())
            .ok_or_else(|| anyhow!("No embedding returned"))
    }
}
