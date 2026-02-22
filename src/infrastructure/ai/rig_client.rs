use crate::application::traits::ai_client::AIClient;
use crate::infrastructure::ai::tools::*;
use crate::shared::config::{Model, Provider};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use rig::completion::Prompt;
use rig::completion::request::PromptError;
use rig::prelude::*;
use rig::{providers, tool::Tool};

pub struct RigClient {
    rig_client: rig::agent::Agent<providers::openai::responses_api::ResponsesCompletionModel>,
}

impl RigClient {
    pub async fn new(api_key: String, provider: Provider, model: Model) -> Result<Self> {
        let system_instruction =
            std::fs::read_to_string("INSTRUCTION.md").context("Failed to read INSTRUCTION.md")?;

        let openai_comp_client = providers::openai::Client::builder()
            .api_key(api_key)
            .base_url(provider.api_url)
            .build()
            .context("Failed to build openai client")?;

        let discord_agent = openai_comp_client
            .agent(model.name)
            .preamble(system_instruction.as_str())
            .tool(test::Test)
            .default_max_turns(10)
            .build();

        Ok(Self {
            rig_client: discord_agent,
        })
    }
}

#[async_trait]
impl AIClient for RigClient {
    async fn new(api_key: String, provider: Provider, model: Model) -> Result<Self> {
        RigClient::new(api_key, provider, model).await
    }

    async fn generate(&self, conversation: String) -> Result<String> {
        self.rig_client
            .prompt(conversation)
            .await
            .map_err(|e: PromptError| anyhow!(e.to_string()))
    }
}
