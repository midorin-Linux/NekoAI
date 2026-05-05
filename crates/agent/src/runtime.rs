use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use dashmap::DashMap;
use nekoai_config::loader::{Config, Parameters};
use nekoai_domain::agent::{
    runtime::{CallerContext, with_caller_context},
    session::SessionKey,
};
use nekoai_memory::{
    short_term::{Role, ShortTermEntry},
    store::MemoryStore,
};
use rig::{
    completion::{Message, Prompt},
    tool::{
        ToolDyn,
        server::{ToolServer, ToolServerHandle},
    },
};
use serde::Deserialize;
use tokio::sync::{Semaphore, mpsc};
use tokio_retry::{
    Retry,
    strategy::{ExponentialBackoff, jitter},
};
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
    pub const TOTAL_STEPS: u64 = 6;

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
    session_manager: Arc<SessionManager>,
    context_manager: Arc<ContextManager>,
    memory_store: Arc<MemoryStore>,
    conversation_model: Arc<OpenAICompatibleAdapter>,
    conversation_model_name: String,
    conversation_model_parameters: Parameters,
    summarization_model: Arc<OpenAICompatibleAdapter>,
    summarization_model_name: String,
    summarization_model_parameters: Parameters,
    extraction_tx: mpsc::Sender<ExtractionTask>,
    tool_server_handle: ToolServerHandle,
    summarizing: Arc<DashMap<SessionKey, ()>>,
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
        let session_manager = Arc::new(SessionManager::new());
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

        let conversation_client = rig::providers::openai::Client::builder()
            .api_key(config.provider.conversation_model.api_key.as_ref())
            .base_url(&config.provider.conversation_model.provider_base_url)
            .build()
            .context("failed to build OpenAI compatible responses client")?
            .completions_api();
        let conversation_model = Arc::new(OpenAICompatibleAdapter::new(conversation_client));

        let summarization_client = rig::providers::openai::Client::builder()
            .api_key(config.provider.summarizer_model.api_key.as_ref())
            .base_url(&config.provider.summarizer_model.provider_base_url)
            .build()
            .context("failed to build OpenAI compatible responses client")?
            .completions_api();
        let summarization_model = Arc::new(OpenAICompatibleAdapter::new(summarization_client));
        on_progress(RuntimeInitProgress::new(
            4,
            "language model provider initialized",
        ));

        info!(
            conversation_model = conversation_model.provider_name(),
            summarzation_model = summarization_model.provider_name(),
            "language model client initialized"
        );

        let conversation_model_name = config.provider.conversation_model.model_name;
        let conversation_model_parameters = config.provider.conversation_model.parameters;
        let summarization_model_name = config.provider.summarizer_model.model_name;
        let summarization_model_parameters = config.provider.summarizer_model.parameters;

        let (extraction_tx, extraction_rx) = mpsc::channel(EXTRACTION_QUEUE_SIZE);

        let semaphore = Arc::new(Semaphore::new(EXTRACTION_CONCURRENT_LIMIT));

        let summarizing = Arc::new(DashMap::new());

        let provider_clone = conversation_model.clone();
        let memory_store_clone = memory_store.clone();
        let conversation_model_name_clone = conversation_model_name.clone();
        let conversation_model_parameters_clone = conversation_model_parameters.clone();
        // let summarization_model_name_clone = summarization_model_name.clone();
        // let summarization_model_parameters_clone = summarization_model_parameters.clone();
        let sem_clone = semaphore.clone();

        tokio::spawn(async move {
            info!("extraction task processor started");
            extraction_task_processor(
                extraction_rx,
                provider_clone,
                memory_store_clone,
                conversation_model_name_clone,
                conversation_model_parameters_clone,
                sem_clone,
            )
            .await;
        });

        let tool_server_handle = ToolServer::new().run();
        on_progress(RuntimeInitProgress::new(5, "tool server initialized"));

        on_progress(RuntimeInitProgress::new(6, "agent runtime initialized"));

        info!("agent runtime initialized");

        Ok(Self {
            session_manager,
            context_manager,
            memory_store,
            conversation_model,
            conversation_model_name,
            conversation_model_parameters,
            summarization_model,
            summarization_model_name,
            summarization_model_parameters,
            extraction_tx,
            tool_server_handle,
            summarizing,
        })
    }

    pub async fn clear_session(&self, session_key: &SessionKey) -> Result<()> {
        let messages = self.memory_store.get_short_term_messages(session_key);

        self.memory_store.clear_short_term(session_key);
        self.session_manager.clear(session_key)?;

        if !messages.is_empty() {
            let this = self.clone();
            let session_key = session_key.clone();
            tokio::spawn(async move {
                match this.generate_mid_term_summary(&messages).await {
                    Ok(summary) => {
                        if let Err(error) = this
                            .memory_store
                            .promote_to_mid_term_with_messages(&session_key, &messages, summary)
                            .await
                        {
                            warn!(
                                session = %session_key.channel_id,
                                error = %error,
                                "failed to store mid-term summary during clear"
                            );
                        }
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

        Ok(())
    }

    pub async fn get_history(&self, session_key: &SessionKey) -> Result<Session> {
        let session = self.session_manager.get(session_key)?;
        Ok(session.lock().await.clone())
    }

    pub async fn submit(
        &self,
        session_key: SessionKey,
        user_id: Option<String>,
        user_input: String,
    ) -> Result<AgentResponse> {
        let caller_context = CallerContext {
            user_id: user_id.as_ref().and_then(|id| id.parse::<u64>().ok()),
            guild_id: session_key.guild_id.map(|id| id.get()),
        };

        info!(
            session = %session_key.channel_id,
            input_len = user_input.len(),
            "submitting user input"
        );
        let session = {
            let session_arc = self.session_manager.get_or_create(&session_key);
            session_arc.lock().await.clone()
        };
        debug!(turn_count = session.turns.len(), "session loaded");

        let recalled = self.memory_store.recall(&session_key, &user_input).await;

        let context = self
            .context_manager
            .build(
                &session,
                &user_input,
                &recalled,
                user_id.clone(),
                session_key.guild_id.map(|id| id.get()),
            )
            .await;
        debug!(context_turns = context.turns.len(), "context built");

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

        let retry_strategy = ExponentialBackoff::from_millis(100)
            .max_delay(Duration::from_secs(10))
            .map(jitter)
            .take(5);

        let cm = self.conversation_model.clone();
        let model_name = self.conversation_model_name.clone();
        let model_params = self.conversation_model_parameters.clone();
        let system_prompt = context.system_prompt.clone();
        let tool_handle = self.tool_server_handle.clone();
        let user_message = context.user_message.clone();

        let result = with_caller_context(caller_context, async {
            Retry::spawn(retry_strategy, || {
                let cm = cm.clone();
                let mn = model_name.clone();
                let mp = model_params.clone();
                let sp = system_prompt.clone();
                let th = tool_handle.clone();
                let um = user_message.clone();
                let ch = chat_history.clone();
                async move {
                    let agent = cm
                        .build_agent(mn.as_str(), mp)
                        .preamble(sp.as_str())
                        .tool_server_handle(th)
                        .build();
                    agent.prompt(&um).max_turns(20).with_history(ch).await
                }
            })
            .await
        })
        .await?;
        info!(response_len = result.len(), "received model response");

        self.memory_store
            .push_short_term(&session_key, &user_input, result.as_str());
        debug!("short-term memory updated");

        if self.memory_store.should_summarize(&session_key)
            && self.summarizing.insert(session_key.clone(), ()).is_none()
        {
            let this = self.clone();
            let key = session_key.clone();
            tokio::spawn(async move {
                if let Err(error) = this
                    .promote_short_term_to_mid_term(&key, "compression_threshold")
                    .await
                {
                    warn!(session = %key.channel_id, error = %error, "background promote failed");
                }
                this.summarizing.remove(&key);
            });
        }

        self.session_manager
            .append(&session_key, &user_input, result.as_str())
            .await;
        debug!("session history updated");

        let conversation = format!(
            "<user_content>{}<user_content>\n<assistant_content>{}<assistant_content>\n",
            user_input, result
        );
        self.spawn_long_term_extraction(session_key.clone(), user_id, conversation);

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
        let conversation = escape_xml(&format_short_term_messages(messages));
        let prompt = format!(
            "<summarization_task>\n  <instruction>\n    The following is a conversation log from the same session.\n    - Please retain the main topics, user intent, conclusions reached, and unresolved issues.\n    - Please summarize it concisely in 5-10 sentences, using the original language of the conversation.\n    - Please write in natural prose, not in bullet points.\n  </instruction>\n  <conversation_log>{}</conversation_log>\n</summarization_task>",
            conversation
        );

        let retry_strategy = ExponentialBackoff::from_millis(100)
            .max_delay(Duration::from_secs(10))
            .map(jitter)
            .take(5);

        let sm = self.summarization_model.clone();
        let model_name = self.summarization_model_name.clone();
        let model_params = self.summarization_model_parameters.clone();

        let summary = Retry::spawn(retry_strategy, || {
            let sm = sm.clone();
            let mn = model_name.clone();
            let mp = model_params.clone();
            let p = prompt.clone();
            async move {
                let summarizer = sm.build_agent(mn.as_str(), mp).build();
                summarizer.prompt(p).await
            }
        })
        .await?;
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

    pub async fn add_tool(&self, tool: impl ToolDyn + 'static) {
        if let Err(e) = self.tool_server_handle.add_tool(tool).await {
            warn!(error = %e, "failed to register tool");
        } else {
            info!("tool registered successfully");
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

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
        "<long_term_extraction_task>\n  <instruction>Please output any important information from the following response in JSON format, which should be referenced in future conversations. Otherwise, return an empty array.</instruction>\n  <output_format>[{{\"fact\":\" ... \",\"tags\":[\" ... \"]}}]</output_format>\n  <response>{}</response>\n</long_term extraction_task>",
        escape_xml(&response)
    );

    let retry_strategy = ExponentialBackoff::from_millis(100)
        .max_delay(Duration::from_secs(10))
        .map(jitter)
        .take(5);

    let facts = Retry::spawn(retry_strategy, || {
        let provider = provider.clone();
        let model = model.clone();
        let parameters = parameters.clone();
        let prompt = prompt.clone();
        let session_key = session_key.clone();

        async move {
            let extractor = provider.build_agent(model.as_str(), parameters).build();
            let extracted = extractor
                .prompt(prompt)
                .await
                .context("failed to prompt extraction agent")?;

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
    let parsed: Vec<ExtractedFact> = serde_json::from_str(candidate)
        .map_err(|e| {
            warn!(
                error = %e,
                candidate = candidate,
                "failed to parse extracted facts JSON"
            );
        })
        .ok()?;
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
