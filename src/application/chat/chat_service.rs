use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::{
    application::traits::{
        ai_client::AIClient, long_term_store::LongTermStore, short_term_store::ShortTermStore,
    },
    models::{error::AppError, memory::*},
    shared::config::MemoryConfig,
};

pub async fn process_message(
    ai_client: &dyn AIClient,
    short_term_store: &dyn ShortTermStore,
    long_term_store: &dyn LongTermStore,
    channel_id: u64,
    user_id: u64,
    user_message: String,
    memory_config: &MemoryConfig,
) -> Result<String, AppError> {
    let in_memory_context = short_term_store.get_context(channel_id).await;

    let query_embedding = ai_client
        .embed(user_message.clone())
        .await
        .map_err(|e| AppError::Embedding(e.to_string()))?;

    // 中期・長期メモリの検索を並行実行
    let midterm_limit = memory_config.midterm_search_limit as u64;
    let longterm_limit = memory_config.longterm_search_limit as u64;

    let (midterm_result, longterm_result) = tokio::join!(
        long_term_store.search_midterm(query_embedding.clone(), user_id, midterm_limit),
        long_term_store.search_longterm(query_embedding.clone(), user_id, longterm_limit),
    );

    let midterm_results = midterm_result.map_err(|e| AppError::Store(e.to_string()))?;
    let longterm_results = longterm_result.map_err(|e| AppError::Store(e.to_string()))?;

    let (prompt_message, chat_history) = build_messages(
        &user_message,
        &in_memory_context,
        &midterm_results,
        &longterm_results,
    );

    tracing::debug!("Sending {} messages in chat history", chat_history.len());

    let response = ai_client
        .generate(prompt_message, chat_history)
        .await
        .map_err(|e| AppError::AIGeneration(e.to_string()))?;

    let now = current_timestamp();
    let midterm_expiry_secs = (memory_config.midterm_expiry_days * 24 * 60 * 60) as i64;

    let user_msg = ShortTermMessage {
        role: Role::User,
        user_id,
        content: user_message,
        timestamp: now,
    };
    let overflow = short_term_store.push(channel_id, user_msg).await;
    let fail_count = promote_overflow(
        ai_client,
        long_term_store,
        user_id,
        channel_id,
        overflow,
        midterm_expiry_secs,
    )
    .await;
    if fail_count > 0 {
        tracing::warn!(
            "promote_overflow: {fail_count} user message(s) failed to promote to midterm memory"
        );
    }

    let assistant_msg = ShortTermMessage {
        role: Role::Assistant,
        user_id,
        content: response.clone(),
        timestamp: current_timestamp(),
    };
    let overflow = short_term_store.push(channel_id, assistant_msg).await;
    let fail_count = promote_overflow(
        ai_client,
        long_term_store,
        user_id,
        channel_id,
        overflow,
        midterm_expiry_secs,
    )
    .await;
    if fail_count > 0 {
        tracing::warn!(
            "promote_overflow: {fail_count} assistant message(s) failed to promote to midterm memory"
        );
    }

    Ok(response)
}

async fn promote_overflow(
    ai_client: &dyn AIClient,
    long_term_store: &dyn LongTermStore,
    user_id: u64,
    channel_id: u64,
    overflow: Vec<ShortTermMessage>,
    expiry_secs: i64,
) -> usize {
    let mut fail_count = 0usize;

    for msg in overflow {
        let role_str = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };
        let summary = format!("[{}] {}", role_str, msg.content);
        let embedding = match ai_client.embed(summary.clone()).await {
            Ok(e) => e,
            Err(err) => {
                tracing::warn!("Failed to embed overflow message for midterm: {err}");
                fail_count += 1;
                continue;
            }
        };

        let now = current_timestamp();
        let memory = MidTermMemory {
            id: Uuid::new_v4().to_string(),
            user_id,
            channel_id,
            summary,
            created_at: now,
            expires_at: now + expiry_secs,
        };

        if let Err(err) = long_term_store.store_midterm(memory, embedding).await {
            tracing::warn!("Failed to store midterm memory: {err}");
            fail_count += 1;
        }
    }

    fail_count
}

fn build_messages(
    user_message: &str,
    short_context: &[ShortTermMessage],
    midterm: &[MidTermMemory],
    longterm: &[LongTermMemory],
) -> (ChatMessage, Vec<ChatMessage>) {
    let mut history: Vec<ChatMessage> = Vec::new();

    if !longterm.is_empty() || !midterm.is_empty() {
        let mut context_text = String::new();

        if !longterm.is_empty() {
            context_text.push_str("[What we know about this user]\n");
            for point in longterm {
                context_text.push_str(&format!("- {}\n", point.fact));
            }
            context_text.push('\n');
        }

        if !midterm.is_empty() {
            context_text.push_str("[Summarizing relevant past conversations]\n");
            for point in midterm {
                context_text.push_str(&format!("- {}\n", point.summary));
            }
        }

        history.push(ChatMessage::user(context_text.trim_end().to_string()));
        history.push(ChatMessage::assistant(
            "Understood. I will use this context in our conversation.",
        ));
    }

    for msg in short_context {
        let message = match msg.role {
            Role::Assistant => ChatMessage::assistant(msg.content.clone()),
            Role::User => ChatMessage::user(msg.content.clone()),
        };
        history.push(message);
    }

    let prompt = ChatMessage::user(user_message.to_string());

    (prompt, history)
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_messages_no_context() {
        let (prompt, history) = build_messages("hello", &[], &[], &[]);
        assert_eq!(history.len(), 0);
        assert_eq!(prompt, ChatMessage::user("hello"));
    }

    #[test]
    fn build_messages_with_short_context() {
        let short = vec![
            ShortTermMessage {
                role: Role::User,
                user_id: 1,
                content: "hi".to_string(),
                timestamp: 0,
            },
            ShortTermMessage {
                role: Role::Assistant,
                user_id: 1,
                content: "hello".to_string(),
                timestamp: 1,
            },
        ];

        let (prompt, history) = build_messages("how are you", &short, &[], &[]);
        assert_eq!(history.len(), 2);
        assert_eq!(prompt, ChatMessage::user("how are you"));
    }

    #[test]
    fn build_messages_with_longterm_context() {
        let longterm = vec![LongTermMemory {
            id: "1".to_string(),
            user_id: 1,
            fact: "Likes Rust".to_string(),
            category: "preference".to_string(),
            created_at: 0,
            updated_at: 0,
        }];

        let (_prompt, history) = build_messages("hello", &[], &[], &longterm);
        // Should have context injection pair (user + assistant)
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn build_messages_with_midterm_context() {
        let midterm = vec![MidTermMemory {
            id: "1".to_string(),
            user_id: 1,
            channel_id: 100,
            summary: "Discussed project".to_string(),
            created_at: 0,
            expires_at: 999,
        }];

        let (_prompt, history) = build_messages("hello", &[], &midterm, &[]);
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn build_messages_full_context() {
        let short = vec![ShortTermMessage {
            role: Role::User,
            user_id: 1,
            content: "prev msg".to_string(),
            timestamp: 0,
        }];
        let midterm = vec![MidTermMemory {
            id: "1".to_string(),
            user_id: 1,
            channel_id: 100,
            summary: "Past talk".to_string(),
            created_at: 0,
            expires_at: 999,
        }];
        let longterm = vec![LongTermMemory {
            id: "1".to_string(),
            user_id: 1,
            fact: "Likes cats".to_string(),
            category: "preference".to_string(),
            created_at: 0,
            updated_at: 0,
        }];

        let (prompt, history) = build_messages("new msg", &short, &midterm, &longterm);
        // 2 context injection + 1 short term
        assert_eq!(history.len(), 3);
        assert_eq!(prompt, ChatMessage::user("new msg"));
    }
}
