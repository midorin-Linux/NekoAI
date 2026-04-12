use chrono::{DateTime, Utc};
use domain::agent::session::SessionKey;
use rig::completion::Message;

#[derive(Clone)]
pub struct ConversationTurn {
    pub user: String,
    pub assistant: String,
}

#[derive(Clone)]
pub struct Session {
    pub key: SessionKey,
    pub messages: Vec<Message>,
    pub turns: Vec<ConversationTurn>,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub token_count: usize,
}

pub struct SessionManager {
    sessions: Vec<Session>,
    max_messages: usize,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            max_messages: 40,
        }
    }

    pub fn get_or_create(&mut self, session_key: &SessionKey) -> &mut Session {
        if let Some(index) = self
            .sessions
            .iter()
            .position(|session| session.key == *session_key)
        {
            return &mut self.sessions[index];
        }

        let now = Utc::now();
        self.sessions.push(Session {
            key: session_key.clone(),
            messages: Vec::new(),
            turns: Vec::new(),
            created_at: now,
            last_active: now,
            token_count: 0,
        });

        self.sessions.last_mut().expect("session was just inserted")
    }

    pub fn append(&mut self, session_key: &SessionKey, user: &str, assistant: &str) {
        let max_messages = self.max_messages;
        let session = self.get_or_create(session_key);
        session.turns.push(ConversationTurn {
            user: user.to_string(),
            assistant: assistant.to_string(),
        });
        session.messages.push(Message::user(user));
        session.messages.push(Message::assistant(assistant));

        while session.messages.len() > max_messages {
            session.messages.remove(0);
        }

        while session.turns.len() * 2 > max_messages {
            session.turns.remove(0);
        }

        session.last_active = Utc::now();
    }
}
