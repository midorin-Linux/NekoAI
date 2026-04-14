use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use config::loader::Config;
use domain::agent::session::SessionKey;
use memory::store::MemoryStore;
use rig::completion::Prompt;
use tracing::{debug, info};

use crate::{
    context::ContextManager,
    provider::OpenAICompatibleAdapter,
    session::{Session, SessionManager},
};

pub struct AgentResponse {
    pub content: String,
}

#[derive(Clone)]
pub struct AgentRuntime {
    session_manager: Arc<Mutex<SessionManager>>,
    context_manager: Arc<ContextManager>,
    memory_store: Arc<MemoryStore>,
    provider: Arc<OpenAICompatibleAdapter>,
    // tool_registry: Arc<ToolRegistry>,
    agent_model_name: String,
}

impl AgentRuntime {
    pub async fn new(config: Config, memory_store: MemoryStore) -> Result<Self> {
        info!("initializing agent runtime");
        let session_manager = Arc::new(Mutex::new(SessionManager::new()));

        let system_instruction_path = std::path::Path::new(".config").join("INSTRUCTION.md");
        let system_instruction = std::fs::read_to_string(system_instruction_path)
            .context("failed to read system instruction")?;

        info!(
            "system instruction loaded, length = {}",
            system_instruction.len()
        );

        let context_manager = Arc::new(ContextManager::new(system_instruction, 16384, 0.7));

        let memory_store = Arc::new(memory_store);

        let openai_client = rig::providers::openai::Client::builder()
            .api_key(&config.provider.language_model.api_key)
            .base_url(&config.provider.language_model.provider_base_url)
            .build()
            .context("failed to build OpenRouter client")?;
        let provider = Arc::new(OpenAICompatibleAdapter::new(openai_client));

        info!(provider = provider.provider_name(), "language model client initialized");

        let agent_model_name = config.provider.language_model.model_name;

        info!("agent runtime initialized");

        Ok(Self {
            session_manager,
            context_manager,
            memory_store,
            provider,
            agent_model_name,
        })
    }

    pub fn clear_session(&self, session_key: &SessionKey) -> Result<()> {
        let mut session_manager = self
            .session_manager
            .lock()
            .expect("session manager mutex poisoned");
        session_manager.clear(session_key)
    }

    pub fn get_history(&self, session_key: &SessionKey) -> Result<Session> {
        let mut session_manager = self
            .session_manager
            .lock()
            .expect("session manager mutex poisoned");
        session_manager.get(session_key).map(|s| s.clone())
    }

    pub async fn submit(
        &self,
        session_key: SessionKey,
        user_input: String,
    ) -> Result<AgentResponse> {
        info!(
            session = %session_key.channel_id,
            input_len = user_input.len(),
            "submitting user input"
        );
        let session = {
            let mut session_manager = self
                .session_manager
                .lock()
                .expect("session manager mutex poisoned");
            session_manager.get_or_create(&session_key).clone()
        };
        debug!(turn_count = session.turns.len(), "session loaded");

        let context = self.context_manager.build(&session, &user_input).await;
        debug!(context_turns = context.turns.len(), "context built");

        let agent = self
            .provider
            .build_agent(self.agent_model_name.as_str())
            .build();

        let mut prompt_text = String::new();
        prompt_text.push_str(&format!("System: {}\n\n", context.system_prompt));
        for turn in &context.turns {
            prompt_text.push_str(&format!(
                "User: {}\nAssistant: {}\n",
                turn.user, turn.assistant
            ));
        }
        prompt_text.push_str(&format!("User: {}", context.user_message));

        debug!(prompt_len = prompt_text.len(), "prompt composed");

        let result = agent.prompt(prompt_text).await?;
        info!(response_len = result.len(), "received model response");

        self.memory_store
            .push_short_term(&session_key, &user_input, result.as_str());
        debug!("short-term memory updated");
        {
            let mut session_manager = self
                .session_manager
                .lock()
                .expect("session manager mutex poisoned");
            session_manager.append(&session_key, &user_input, result.as_str());
        }
        debug!("session history updated");

        Ok(AgentResponse { content: result })
    }
}
