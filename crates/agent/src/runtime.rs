use std::sync::Arc;

use anyhow::{Context, Result};
use nekoai_config::loader::{Config, Parameters};
use nekoai_domain::agent::session::SessionKey;
use nekoai_memory::{
    short_term::{Role, ShortTermEntry},
    store::MemoryStore,
};
use rig::completion::{Chat, Message, Prompt};
use serde::Deserialize;
use tokio::sync::{Mutex, Semaphore, mpsc};
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

struct ExtractionTask {
    session_key: SessionKey,
    user_id: Option<String>,
    response: String,
}

const EXTRACTION_QUEUE_SIZE: usize = 100;
const EXTRACTION_CONCURRENT_LIMIT: usize = 3;

#[derive(Clone)]
pub struct AgentRuntime {
    session_manager: Arc<Mutex<SessionManager>>,
    context_manager: Arc<ContextManager>,
    memory_store: Arc<MemoryStore>,
    provider: Arc<OpenAICompatibleAdapter>,
    agent_model_name: String,
    agent_parameters: Parameters,
    extraction_tx: mpsc::Sender<ExtractionTask>,
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
            .api_key(config.provider.language_model.api_key.as_ref())
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

        let (extraction_tx, extraction_rx) = mpsc::channel(EXTRACTION_QUEUE_SIZE);

        let semaphore = Arc::new(Semaphore::new(EXTRACTION_CONCURRENT_LIMIT));

        let provider_clone = provider.clone();
        let memory_store_clone = memory_store.clone();
        let model_clone = agent_model_name.clone();
        let parameters_clone = agent_parameters.clone();
        let sem_clone = semaphore.clone();

        tokio::spawn(async move {
            info!("extraction task processor started");
            extraction_task_processor(
                extraction_rx,
                provider_clone,
                memory_store_clone,
                model_clone,
                parameters_clone,
                sem_clone,
            )
            .await;
        });

        on_progress(RuntimeInitProgress::new(5, "agent runtime initialized"));

        info!("agent runtime initialized");

        Ok(Self {
            session_manager,
            context_manager,
            memory_store,
            provider,
            agent_model_name,
            agent_parameters,
            extraction_tx,
        })
    }

    pub async fn clear_session(&self, session_key: &SessionKey) -> Result<()> {
        // Capture messages before clearing so background summarization has data.
        let messages = self.memory_store.get_short_term_messages(session_key);

        if !messages.is_empty() {
            let this = self.clone();
            let session_key = session_key.clone();
            tokio::spawn(async move {
                match this.generate_mid_term_summary(&messages).await {
                    Ok(summary) => {
                        this.memory_store
                            .promote_to_mid_term(&session_key, summary)
                            .await
                            .unwrap_or_else(|error| {
                                warn!(
                                    session = %session_key.channel_id,
                                    error = %error,
                                    "failed to store mid-term summary during clear"
                                );
                            });
                    }
                    Err(error) => {
                        warn!(
                            session = %session_key.channel_id,
                            error = %error,
                            "failed to generate mid-term summary during clear"
                        );
                    }
                }
            });
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
            .preamble(context.system_prompt.as_str())
            .build();

        let mut chat_history = Vec::with_capacity(context.turns.len() * 2);
        for turn in &context.turns {
            chat_history.push(Message::user(&turn.user));
            chat_history.push(Message::assistant(&turn.assistant));
        }

        debug!(
            chat_history_message_count = chat_history.len(),
            context_turn_count = context.turns.len(),
            "prompt composed"
        );

        let result = agent
            .chat(context.user_message.as_str(), chat_history)
            .await?;
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
        let task = ExtractionTask {
            session_key,
            user_id,
            response,
        };

        if let Err(e) = self.extraction_tx.try_send(task) {
            warn!(
                error = %e,
                "failed to queue long-term memory extraction task (queue may be full)"
            );
        }
    }

    pub async fn shutdown(&self) {
        info!("shutting down agent runtime...");

        drop(self.extraction_tx.clone());

        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        info!("agent runtime shutdown complete");
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
    let prompt = format!(
        "以下の応答から、将来の会話で参照すべき重要な情報があれば JSON で出力してください。\nなければ空配列を返してください。\n\n形式: [{{\"fact\": \"...\", \"tags\": [\"...\"]}}]\n\n応答: {}",
        response
    );

    let retry_strategy = tokio_retry::strategy::FixedInterval::from_millis(1000).take(1);

    let facts = tokio_retry::Retry::spawn(retry_strategy, || {
        let provider = provider.clone();
        let model = model.clone();
        let parameters = parameters.clone();
        let prompt = prompt.clone();
        let session_key = session_key.clone();

        async move {
            let extractor = provider.build_agent(model.as_str(), parameters).build();
            let extracted = extractor.prompt(prompt).await.context("failed to prompt extraction agent")?;
            
            match parse_extracted_facts(&extracted) {
                Ok(facts) => Ok(facts),
                Err(e) => {
                    warn!(
                        session = %session_key.channel_id,
                        error = %e,
                        extracted = %extracted,
                        "JSON parse failed, retrying long-term memory extraction"
                    );
                    Err(e)
                }
            }
        }
    })
    .await?;

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

async fn extraction_task_processor(
    mut rx: mpsc::Receiver<ExtractionTask>,
    provider: Arc<OpenAICompatibleAdapter>,
    memory_store: Arc<MemoryStore>,
    model: String,
    parameters: Parameters,
    semaphore: Arc<Semaphore>,
) {
    while let Some(task) = rx.recv().await {
        let provider = provider.clone();
        let memory_store = memory_store.clone();
        let model = model.clone();
        let parameters = parameters.clone();
        let sem = semaphore.clone();

        tokio::spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(permit) => permit,
                Err(e) => {
                    warn!(error = %e, "failed to acquire semaphore permit");
                    return;
                }
            };

            if let Err(error) = extract_and_store_long_term_facts(
                provider,
                memory_store,
                model,
                parameters,
                task.session_key,
                task.user_id,
                task.response,
            )
            .await
            {
                warn!(error = %error, "failed to extract long-term memory facts");
            }
        });
    }

    info!("extraction task processor stopped");
}

fn parse_extracted_facts(raw: &str) -> Result<Vec<(String, Vec<String>)>> {
    parse_extracted_facts_json(raw)
        .or_else(|| {
            let trimmed = raw.trim();
            let start = trimmed.find('[')?;
            let end = trimmed.rfind(']')?;

            if end < start {
                return None;
            }

            parse_extracted_facts_json(&trimmed[start ..= end])
        })
        .ok_or_else(|| anyhow::anyhow!("failed to parse extracted facts JSON: {}", raw))
}

fn parse_extracted_facts_json(candidate: &str) -> Option<Vec<(String, Vec<String>)>> {
    let parsed: Vec<ExtractedFact> = serde_json::from_str(candidate).map_err(|e| {
        warn!(
            error = %e,
            candidate = candidate,
            "failed to parse extracted facts JSON"
        );
    }).ok()?;
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
