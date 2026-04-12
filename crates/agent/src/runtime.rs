use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use config::loader::Config;
use domain::agent::session::SessionKey;
use memory::store::MemoryStore;
use rig::completion::Prompt;

use crate::{context::ContextManager, provider::OpenRouterAdapter, session::SessionManager};

pub struct AgentResponse {
    pub content: String,
}

pub struct AgentRuntime {
    session_manager: Arc<Mutex<SessionManager>>,
    context_manager: Arc<ContextManager>,
    memory_store: Arc<MemoryStore>,
    provider: Arc<OpenRouterAdapter>,
    // tool_registry: Arc<ToolRegistry>,
}

impl AgentRuntime {
    pub async fn new(config: Config, memory_store: MemoryStore) -> Result<Self> {
        let session_manager = Arc::new(Mutex::new(SessionManager::new()));

        let context_manager = Arc::new(ContextManager::new(
            "You are helpful assistant.".to_string(),
            16384,
            0.7,
        ));

        let openai_client = rig::providers::openrouter::Client::builder()
            .api_key(&config.provider.language_model.api_key)
            .build()
            .context("failed to build OpenRouter client")?;

        let memory_store = Arc::new(memory_store);

        let provider = Arc::new(OpenRouterAdapter::new(openai_client));

        Ok(Self {
            session_manager,
            context_manager,
            memory_store,
            provider,
        })
    }

    pub async fn submit(
        &self,
        session_key: SessionKey,
        user_input: String,
    ) -> Result<AgentResponse> {
        let session = {
            let mut session_manager = self
                .session_manager
                .lock()
                .expect("session manager mutex poisoned");
            session_manager.get_or_create(&session_key).clone()
        };

        let context = self.context_manager.build(&session, &user_input).await;

        let agent = self
            .provider
            .build_agent("nvidia/nemotron-3-super-120b-a12b:free")
            .preamble("You are helpful assistant.")
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

        let result = agent.prompt(prompt_text).await?;

        self.memory_store
            .push_short_term(&session_key, &user_input, result.as_str());
        {
            let mut session_manager = self
                .session_manager
                .lock()
                .expect("session manager mutex poisoned");
            session_manager.append(&session_key, &user_input, result.as_str());
        }

        Ok(AgentResponse { content: result })
    }
}
