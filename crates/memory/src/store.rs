use domain::agent::session::SessionKey;

use crate::short_term::ShortTermMemory;

pub struct MemoryStore {
    short_term_memory: ShortTermMemory,
}

impl MemoryStore {
    pub fn new() -> Self {
        let short_term_memory = ShortTermMemory::new(20);

        Self { short_term_memory }
    }

    pub fn push_short_term(&self, session_key: &SessionKey, user: &str, assistant: &str) {
        self.short_term_memory
            .push_turn(session_key, user, assistant);
    }
}
