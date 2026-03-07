use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use rig::{
    completion::{Chat, Message, request::PromptError},
    embeddings::EmbeddingModel,
    prelude::*,
    providers,
};
use serenity::all::Http;

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
    ) -> Result<Self> {
        let system_instruction =
            std::fs::read_to_string("INSTRUCTION.md").context("Failed to read INSTRUCTION.md")?;

        let openai_comp_nlp_client = providers::openai::Client::builder()
            .api_key(nlp_api_key)
            .base_url(nlp.api_url)
            .build()
            .context("Failed to build openai nlp client")?;

        let nlp_client = openai_comp_nlp_client
            .agent(nlp.model_name)
            .preamble(system_instruction.as_str())
            .tool(send_message::SendMessage::new(discord_http.clone()))
            .default_max_turns(10)
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
