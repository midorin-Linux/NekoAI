use crate::session::{ConversationTurn, Session};

pub struct Context {
    pub system_prompt: String,
    pub turns: Vec<ConversationTurn>,
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

    pub async fn build(&self, session: &Session, input: &str) -> Context {
        let mut turns = session.turns.clone();

        let max_turns = (self.max_tokens / 512).max(1);
        if turns.len() > max_turns {
            let drain_count = turns.len() - max_turns;
            turns.drain(0 .. drain_count);
        }

        let system_prompt = self.base_system_prompt.clone();

        Context {
            system_prompt,
            turns,
            user_message: input.to_string(),
        }
    }
}
