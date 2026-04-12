use domain::agent::session::SessionKey;
use tracing::{debug, info};

use crate::short_term::ShortTermMemory;

pub struct MemoryStore {
    short_term_memory: ShortTermMemory,
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore {
    pub fn new() -> Self {
        let short_term_memory = ShortTermMemory::new(20);
        info!("memory store initialized");

        Self { short_term_memory }
    }

    pub fn push_short_term(&self, session_key: &SessionKey, user: &str, assistant: &str) {
        debug!(
            session = %session_key.channel_id,
            user_len = user.len(),
            assistant_len = assistant.len(),
            "pushing conversation turn to short-term memory"
        );
        self.short_term_memory
            .push_turn(session_key, user, assistant);
    }
}
