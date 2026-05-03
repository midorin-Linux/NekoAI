use std::collections::VecDeque;

use nekoai_memory::store::RecalledMemory;
use tracing::debug;

use crate::session::{ConversationTurn, Session};

pub struct Context {
    pub system_prompt: String,
    pub turns: VecDeque<ConversationTurn>,
    pub user_message: String,
}

pub struct ContextManager {
    base_system_prompt: String,
    max_tokens: usize,
    #[allow(dead_code)]
    compaction_threshold: f32,
}

impl ContextManager {
    pub fn new(base_system_prompt: String, max_tokens: usize, compaction_threshold: f32) -> Self {
        Self {
            base_system_prompt,
            max_tokens,
            compaction_threshold,
        }
    }

    pub async fn build(
        &self,
        session: &Session,
        input: &str,
        recalled_memory: &RecalledMemory,
        caller_user_id: Option<String>,
        caller_guild_id: Option<u64>,
    ) -> Context {
        debug!(
            input_len = input.len(),
            session_turns = session.turns.len(),
            max_tokens = self.max_tokens,
            "building prompt context"
        );
        let mut turns = session.turns.clone();

        let max_turns = (self.max_tokens / 512).max(1);
        if turns.len() > max_turns {
            let drain_count = turns.len() - max_turns;
            for _ in 0 .. drain_count {
                turns.pop_front();
            }
            debug!(
                drained_turns = drain_count,
                "compacted conversation turns for context"
            );
        }

        let system_prompt =
            self.build_system_prompt_with_memory(recalled_memory, caller_user_id, caller_guild_id);

        Context {
            system_prompt,
            turns,
            user_message: input.to_string(),
        }
    }

    fn build_system_prompt_with_memory(
        &self,
        recalled: &RecalledMemory,
        caller_user_id: Option<String>,
        caller_guild_id: Option<u64>,
    ) -> String {
        let mut prompt = self.base_system_prompt.clone();

        let user_id = caller_user_id.unwrap_or_else(|| "unknown".to_string());
        let guild_id = caller_guild_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let channel_id = recalled
            .mid_term
            .first()
            .map(|_| "unknown".to_string())
            .unwrap_or_else(|| "unknown".to_string());

        prompt = prompt
            .replace("{guild_name}", "unknown")
            .replace("{channel_name}", "unknown")
            .replace("{category}", "unknown")
            .replace("{user_name}", "unknown")
            .replace("{user_id}", &user_id)
            .replace("{guild_id}", &guild_id)
            .replace("{channel_id}", &channel_id)
            .replace("{roles}", "unknown");

        if !recalled.long_term.is_empty() {
            prompt.push_str("\n\n<ImportantMemories>\n");
            for mem in &recalled.long_term {
                prompt.push_str(&format!("  <Memory>{}</Memory>\n", mem.content));
            }
            prompt.push_str("\n\n</ImportantMemories>\n");
        }

        if !recalled.mid_term.is_empty() {
            prompt.push_str("\n\n<PastConversations>\n");
            for summary in &recalled.mid_term {
                prompt.push_str(&format!(
                    "<Conversation>{}</Conversation>\n",
                    summary.content
                ));
            }
            prompt.push_str("\n\n</PastConversations>\n");
        }

        prompt
    }
}
