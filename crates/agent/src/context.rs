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

        let system_prompt = self.build_system_prompt_with_memory(recalled_memory);

        Context {
            system_prompt,
            turns,
            user_message: input.to_string(),
        }
    }

    fn build_system_prompt_with_memory(&self, recalled: &RecalledMemory) -> String {
        let mut prompt = self.base_system_prompt.clone();

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
