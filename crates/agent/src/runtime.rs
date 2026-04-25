use std::sync::Arc;

use anyhow::{Context, Result};
use nekoai_config::loader::{Config, Parameters};
use nekoai_domain::agent::session::SessionKey;
use nekoai_memory::{
    short_term::{Role, ShortTermEntry},
    store::MemoryStore,
};
use rig::completion::Prompt;
use serde::Deserialize;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::{
    context::ContextManager,
    provider::OpenAICompatibleAdapter,
    session::{Session, SessionManager},
};

#[derive(Debug, Clone, Copy)]
pub struct RuntimeInitProgress {
    pub completed_steps: u64,
    pub total_steps: u64,
    pub message: &'static str,
}

impl RuntimeInitProgress {
    pub const TOTAL_STEPS: u64 = 5;

    fn new(completed_steps: u64, message: &'static str) -> Self {
        Self {
            completed_steps,
            total_steps: Self::TOTAL_STEPS,
            message,
        }
    }
}

pub struct AgentResponse {
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ExtractedFact {
    fact: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Clone)]
pub struct AgentRuntime {
    session_manager: Arc<Mutex<SessionManager>>,
    context_manager: Arc<ContextManager>,
    memory_store: Arc<MemoryStore>,
    provider: Arc<OpenAICompatibleAdapter>,
    // tool_registry: Arc<ToolRegistry>,
    agent_model_name: String,
    agent_parameters: Parameters,
}

impl AgentRuntime {
    pub async fn new(config: Config, memory_store: MemoryStore) -> Result<Self> {
        Self::new_with_progress(config, memory_store, |_| {}).await
    }

    pub async fn new_with_progress<F>(
        config: Config,
        memory_store: MemoryStore,
        mut on_progress: F,
    ) -> Result<Self>
    where
        F: FnMut(RuntimeInitProgress),
    {
        info!("initializing agent runtime");
        let session_manager = Arc::new(Mutex::new(SessionManager::new()));
        on_progress(RuntimeInitProgress::new(1, "session manager ready"));

        let system_instruction_path = std::path::Path::new(".config").join("INSTRUCTION.md");
        let system_instruction = std::fs::read_to_string(system_instruction_path)
            .context("failed to read system instruction")?;
        on_progress(RuntimeInitProgress::new(2, "system instruction loaded"));

        info!(
            "system instruction loaded, length = {}",
            system_instruction.len()
        );

        let context_manager = Arc::new(ContextManager::new(system_instruction, 16384, 0.7));

        let memory_store = Arc::new(memory_store);
        on_progress(RuntimeInitProgress::new(
            3,
            "context manager and memory store ready",
        ));

        let openai_client = rig::providers::openai::Client::builder()
            .api_key(&config.provider.language_model.api_key)
            .base_url(&config.provider.language_model.provider_base_url)
            .build()
            .context("failed to build OpenAI compatible responses client")?
            .completions_api();
        let provider = Arc::new(OpenAICompatibleAdapter::new(openai_client));
        on_progress(RuntimeInitProgress::new(
            4,
            "language model provider initialized",
        ));

        info!(
            provider = provider.provider_name(),
            "language model client initialized"
        );

        let agent_model_name = config.provider.language_model.model_name;
        let agent_parameters = config.provider.language_model.parameters;

        info!("agent runtime initialized");
        on_progress(RuntimeInitProgress::new(5, "agent runtime initialized"));

        Ok(Self {
            session_manager,
            context_manager,
            memory_store,
            provider,
            agent_model_name,
            agent_parameters,
        })
    }

    pub async fn clear_session(&self, session_key: &SessionKey) -> Result<()> {
        if let Err(error) = self
            .promote_short_term_to_mid_term(session_key, "clear_command")
            .await
        {
            warn!(
                session = %session_key.channel_id,
                error = %error,
                "failed to promote short-term memory before clearing session"
            );
        }

        self.memory_store.clear_short_term(session_key);

        let mut session_manager = self.session_manager.lock().await;
        session_manager.clear(session_key)
    }

    pub async fn get_history(&self, session_key: &SessionKey) -> Result<Session> {
        let mut session_manager = self.session_manager.lock().await;
        session_manager.get(session_key).map(|s| s.clone())
    }

    pub async fn submit(
        &self,
        session_key: SessionKey,
        user_id: Option<String>,
        user_input: String,
    ) -> Result<AgentResponse> {
        info!(
            session = %session_key.channel_id,
            input_len = user_input.len(),
            "submitting user input"
        );
        let session = {
            let mut session_manager = self.session_manager.lock().await;
            session_manager.get_or_create(&session_key).clone()
        };
        debug!(turn_count = session.turns.len(), "session loaded");

        let recalled = self.memory_store.recall(&session_key, &user_input).await;

        let context = self
            .context_manager
            .build(&session, &user_input, &recalled)
            .await;
        debug!(context_turns = context.turns.len(), "context built");

        let agent = self
            .provider
            .build_agent(
                self.agent_model_name.as_str(),
                self.agent_parameters.clone(),
            )
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

        if self.memory_store.should_summarize(&session_key)
            && let Err(error) = self
                .promote_short_term_to_mid_term(&session_key, "compression_threshold")
                .await
        {
            warn!(
                session = %session_key.channel_id,
                error = %error,
                "failed to promote short-term memory at compression threshold"
            );
        }

        {
            let mut session_manager = self.session_manager.lock().await;
            session_manager.append(&session_key, &user_input, result.as_str());
        }
        debug!("session history updated");

        self.spawn_long_term_extraction(session_key.clone(), user_id, result.clone());

        Ok(AgentResponse { content: result })
    }

    async fn promote_short_term_to_mid_term(
        &self,
        session_key: &SessionKey,
        trigger: &'static str,
    ) -> Result<()> {
        let messages = self.memory_store.get_short_term_messages(session_key);
        if messages.is_empty() {
            debug!(
                session = %session_key.channel_id,
                trigger = trigger,
                "skipped mid-term promotion because short-term memory is empty"
            );
            return Ok(());
        }

        let summary = self.generate_mid_term_summary(&messages).await?;
        self.memory_store
            .promote_to_mid_term(session_key, summary)
            .await
            .context("failed to store summary in mid-term memory")?;

        info!(
            session = %session_key.channel_id,
            trigger = trigger,
            message_count = messages.len(),
            "promoted short-term memory to mid-term"
        );

        Ok(())
    }

    async fn generate_mid_term_summary(&self, messages: &[ShortTermEntry]) -> Result<String> {
        let conversation = format_short_term_messages(messages);
        let prompt = format!(
            "以下は同一セッションの会話ログです。\n会話の流れと決定事項を保ちつつ、この会話の要点を3文で要約してください。\n箇条書きではなく自然な文章で出力し、3文以外は出力しないでください。\n\n会話ログ:\n{}",
            conversation
        );

        let summarizer = self
            .provider
            .build_agent(
                self.agent_model_name.as_str(),
                self.agent_parameters.clone(),
            )
            .build();

        let summary = summarizer.prompt(prompt).await?;
        Ok(summary.trim().to_string())
    }

    fn spawn_long_term_extraction(
        &self,
        session_key: SessionKey,
        user_id: Option<String>,
        response: String,
    ) {
        let provider = self.provider.clone();
        let memory_store = self.memory_store.clone();
        let model = self.agent_model_name.clone();
        let parameters = self.agent_parameters.clone();

        tokio::spawn(async move {
            if let Err(error) = extract_and_store_long_term_facts(
                provider,
                memory_store,
                model,
                parameters,
                session_key,
                user_id,
                response,
            )
            .await
            {
                warn!(error = %error, "failed to extract long-term memory facts");
            }
        });
    }
}

fn format_short_term_messages(messages: &[ShortTermEntry]) -> String {
    let mut formatted = String::new();

    for entry in messages {
        formatted.push_str(role_label(&entry.role));
        formatted.push_str(": ");
        formatted.push_str(entry.content.trim());
        formatted.push('\n');
    }

    formatted
}

fn role_label(role: &Role) -> &'static str {
    match role {
        Role::User => "User",
        Role::Assistant => "Assistant",
        Role::Tool => "Tool",
    }
}

async fn extract_and_store_long_term_facts(
    provider: Arc<OpenAICompatibleAdapter>,
    memory_store: Arc<MemoryStore>,
    model: String,
    parameters: Parameters,
    session_key: SessionKey,
    user_id: Option<String>,
    response: String,
) -> Result<()> {
    let extractor = provider.build_agent(model.as_str(), parameters).build();
    let prompt = format!(
        "以下の応答から、将来の会話で参照すべき重要な情報があれば JSON で出力してください。\nなければ空配列を返してください。\n\n形式: [{{\"fact\": \"...\", \"tags\": [\"...\"]}}]\n\n応答: {}",
        response
    );

    let extracted = extractor.prompt(prompt).await?;
    let facts = parse_extracted_facts(&extracted);

    if facts.is_empty() {
        debug!(
            session = %session_key.channel_id,
            "no long-term facts were extracted"
        );
        return Ok(());
    }

    let fact_count = facts.len();
    memory_store
        .extract_long_term(&session_key, user_id.as_deref(), facts)
        .await
        .context("failed to store extracted long-term facts")?;

    info!(
        session = %session_key.channel_id,
        fact_count = fact_count,
        "stored extracted long-term facts"
    );

    Ok(())
}

fn parse_extracted_facts(raw: &str) -> Vec<(String, Vec<String>)> {
    parse_extracted_facts_json(raw)
        .or_else(|| {
            let trimmed = raw.trim();
            let start = trimmed.find('[')?;
            let end = trimmed.rfind(']')?;

            if end < start {
                return None;
            }

            parse_extracted_facts_json(&trimmed[start..=end])
        })
        .unwrap_or_default()
}

fn parse_extracted_facts_json(candidate: &str) -> Option<Vec<(String, Vec<String>)>> {
    let parsed: Vec<ExtractedFact> = serde_json::from_str(candidate).ok()?;
    let facts = parsed
        .into_iter()
        .filter_map(|item| {
            let fact = item.fact.trim();
            if fact.is_empty() {
                return None;
            }

            Some((fact.to_string(), item.tags))
        })
        .collect();

    Some(facts)
}
