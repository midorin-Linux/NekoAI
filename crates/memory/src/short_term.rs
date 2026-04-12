use std::collections::VecDeque;

use chrono::Utc;
use dashmap::DashMap;
use domain::agent::session::SessionKey;
use tracing::debug;

#[derive(Debug, Clone)]
pub enum Role {
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone)]
pub struct ShortTermEntry {
    pub role: Role,
    pub content: String,
    pub timestamp: i64,
}

pub struct ShortTermMemory {
    store: DashMap<SessionKey, VecDeque<ShortTermEntry>>,
    max_entry: usize,
}

impl ShortTermMemory {
    pub fn new(max_entry: usize) -> Self {
        Self {
            store: DashMap::new(),
            max_entry,
        }
    }

    pub fn push_turn(&self, session_key: &SessionKey, user: &str, assistant: &str) {
        debug!(
            session = %session_key.channel_id,
            max_entry = self.max_entry,
            "storing short-term conversation turn"
        );
        let mut query = self
            .store
            .entry(session_key.clone())
            .or_insert_with(VecDeque::new);
        query.push_back(ShortTermEntry {
            role: Role::User,
            content: user.to_string(),
            timestamp: Utc::now().timestamp(),
        });
        query.push_back(ShortTermEntry {
            role: Role::Assistant,
            content: assistant.to_string(),
            timestamp: Utc::now().timestamp(),
        });

        while query.len() > self.max_entry {
            query.pop_front();
        }

        debug!(session = %session_key.channel_id, entry_count = query.len(), "short-term memory updated");
    }
}
