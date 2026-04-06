use std::{collections::HashMap, sync::Arc, time::Duration};

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::all::{ChannelId, Http};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::error::DiscordToolError;

/// リマインダーのデータ
#[derive(Debug, Clone)]
pub struct ReminderData {
    pub id: String,
    pub channel_id: u64,
    pub message: String,
    pub delay_secs: u64,
    pub created_by: String,
}

/// リマインダーストア（インメモリ）
pub type ReminderStore = Arc<RwLock<HashMap<String, ReminderData>>>;

/// リマインダーストアを新規作成する
pub fn new_reminder_store() -> ReminderStore {
    Arc::new(RwLock::new(HashMap::new()))
}

// ── SetReminder ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SetReminderArgs {
    channel_id: u64,
    message: String,
    delay_seconds: u64,
}

#[derive(Serialize, Deserialize)]
pub struct SetReminder {
    #[serde(skip)]
    http: Option<Arc<Http>>,
    #[serde(skip)]
    store: Option<ReminderStore>,
}

impl SetReminder {
    pub fn new(http: Arc<Http>, store: ReminderStore) -> Self {
        Self {
            http: Some(http),
            store: Some(store),
        }
    }
}

impl Tool for SetReminder {
    const NAME: &'static str = "set_reminder";
    type Error = DiscordToolError;
    type Args = SetReminderArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "set_reminder".to_string(),
            description: "Set a reminder that will send a message to a channel after the specified delay. Reminders are stored in memory and will be lost on bot restart.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "channel_id": {
                        "type": "integer",
                        "description": "The channel ID to send the reminder to"
                    },
                    "message": {
                        "type": "string",
                        "description": "The reminder message content"
                    },
                    "delay_seconds": {
                        "type": "integer",
                        "description": "Delay in seconds before the reminder is sent (max: 86400 = 24 hours)"
                    }
                },
                "required": ["channel_id", "message", "delay_seconds"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let http = self
            .http
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("set_reminder", "HTTP client not available"))?
            .clone();

        let store = self
            .store
            .as_ref()
            .ok_or_else(|| DiscordToolError::new("set_reminder", "Reminder store not available"))?
            .clone();

        if args.delay_seconds == 0 || args.delay_seconds > 86400 {
            return Err(DiscordToolError::invalid_argument(
                "set_reminder",
                "delay_seconds must be between 1 and 86400 (24 hours)",
            ));
        }

        let id = Uuid::new_v4().to_string();
        let reminder = ReminderData {
            id: id.clone(),
            channel_id: args.channel_id,
            message: args.message.clone(),
            delay_secs: args.delay_seconds,
            created_by: "AI".to_string(),
        };

        {
            let mut s = store.write().await;
            s.insert(id.clone(), reminder);
        }

        let reminder_id = id.clone();
        let channel_id = args.channel_id;
        let message = args.message;
        let delay = args.delay_seconds;

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(delay)).await;

            let cid = ChannelId::new(channel_id);
            let reminder_msg = format!("\u{23f0} **Reminder**: {}", message);
            if let Err(e) = cid.say(http.as_ref(), &reminder_msg).await {
                tracing::error!("Failed to send reminder {}: {}", reminder_id, e);
            }

            let mut s = store.write().await;
            s.remove(&reminder_id);
        });

        Ok(format!(
            "Reminder set (ID: {}). Will fire in {} seconds in channel {}.",
            id, delay, channel_id
        ))
    }
}

// ── ListReminders ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListRemindersArgs {}

#[derive(Serialize, Deserialize)]
pub struct ListReminders {
    #[serde(skip)]
    store: Option<ReminderStore>,
}

impl ListReminders {
    pub fn new(store: ReminderStore) -> Self {
        Self { store: Some(store) }
    }
}

impl Tool for ListReminders {
    const NAME: &'static str = "list_reminders";
    type Error = DiscordToolError;
    type Args = ListRemindersArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "list_reminders".to_string(),
            description: "List all currently active reminders.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let store = self.store.as_ref().ok_or_else(|| {
            DiscordToolError::new("list_reminders", "Reminder store not available")
        })?;

        let s = store.read().await;

        if s.is_empty() {
            return Ok("No active reminders.".to_string());
        }

        let mut result = format!("Active reminders ({}):\n", s.len());
        for (id, reminder) in s.iter() {
            result.push_str(&format!(
                "- ID: {} | Channel: {} | Message: '{}'\n",
                id, reminder.channel_id, reminder.message
            ));
        }

        Ok(result)
    }
}

// ── CancelReminder ──────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CancelReminderArgs {
    reminder_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct CancelReminder {
    #[serde(skip)]
    store: Option<ReminderStore>,
}

impl CancelReminder {
    pub fn new(store: ReminderStore) -> Self {
        Self { store: Some(store) }
    }
}

impl Tool for CancelReminder {
    const NAME: &'static str = "cancel_reminder";
    type Error = DiscordToolError;
    type Args = CancelReminderArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "cancel_reminder".to_string(),
            description: "Cancel a reminder by its ID. Note: the scheduled task may still fire if timing is close.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "reminder_id": {
                        "type": "string",
                        "description": "The reminder ID to cancel"
                    }
                },
                "required": ["reminder_id"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let store = self.store.as_ref().ok_or_else(|| {
            DiscordToolError::new("cancel_reminder", "Reminder store not available")
        })?;

        let mut s = store.write().await;

        if s.remove(&args.reminder_id).is_some() {
            Ok(format!("Reminder {} has been cancelled.", args.reminder_id))
        } else {
            Err(DiscordToolError::not_found(
                "cancel_reminder",
                format!("Reminder {} not found", args.reminder_id),
            ))
        }
    }
}
