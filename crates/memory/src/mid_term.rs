use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use chrono::{DateTime, Utc};
use nekoai_domain::agent::session::{SessionKey, SessionKind};
use serde_json::json;
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    embedding::Embedder,
    short_term::ShortTermEntry,
    store::MemoryEntry,
    vector_db::{FilterCondition, SearchFilter, VectorDbClient},
};

pub struct MidTermMemory {
    db: Arc<dyn VectorDbClient>,
    embedder: Arc<dyn Embedder>,
    collection: String,
    retention_days: u32,
}

impl MidTermMemory {
    pub fn new(
        db: Arc<dyn VectorDbClient>,
        embedder: Arc<dyn Embedder>,
        collection: String,
        retention_days: u32,
    ) -> Self {
        Self {
            db,
            embedder,
            collection,
            retention_days,
        }
    }

    pub fn retention_days(&self) -> u32 {
        self.retention_days
    }

    pub async fn ensure_collection(&self, dim: usize) -> Result<()> {
        self.db.ensure_collection(&self.collection, dim).await?;
        info!(collection = %self.collection, retention_days = self.retention_days, "mid-term memory initialized");
        Ok(())
    }

    pub async fn store_summary(
        &self,
        session_key: &SessionKey,
        messages: &[ShortTermEntry],
        summary: String,
    ) -> Result<()> {
        let embedding = self.embedder.embed(&summary).await;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        let mut payload = HashMap::new();
        payload.insert("content".to_string(), json!(summary));
        payload.insert(
            "guild_id".to_string(),
            json!(session_key.guild_id.map(|g| g.to_string())),
        );
        payload.insert(
            "channel_id".to_string(),
            json!(session_key.channel_id.to_string()),
        );
        payload.insert(
            "kind".to_string(),
            json!(session_kind_value(&session_key.kind)),
        );
        payload.insert("created_at".to_string(), json!(now));
        payload.insert("message_count".to_string(), json!(messages.len()));

        self.db
            .upsert(crate::vector_db::UpsertRequest {
                collection: &self.collection,
                id: &id,
                vector: embedding,
                payload,
            })
            .await?;

        debug!(id = %id, session = %session_key.channel_id, "stored mid-term summary");
        Ok(())
    }

    pub async fn search(
        &self,
        session_key: &SessionKey,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<MemoryEntry>> {
        let embedding = self.embedder.embed(query).await;

        let filter = session_scope_filter(session_key);

        let results = self
            .db
            .search(crate::vector_db::SearchRequest {
                collection: &self.collection,
                vector: embedding,
                filter: Some(filter),
                top_k,
            })
            .await?;

        let entries = results
            .into_iter()
            .map(|r| {
                let content = r
                    .payload
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let created_at = r
                    .payload
                    .get("created_at")
                    .and_then(|v| v.as_i64())
                    .map(|ts| DateTime::from_timestamp(ts, 0).unwrap_or_default())
                    .unwrap_or_default();

                MemoryEntry {
                    content,
                    score: r.score,
                    created_at,
                    metadata: r.payload,
                }
            })
            .collect();

        Ok(entries)
    }

    pub async fn delete_old_entries(&self) -> Result<u64> {
        let cutoff = Utc::now().timestamp() - (self.retention_days as i64 * 24 * 60 * 60);
        let filter = SearchFilter {
            must: vec![FilterCondition::Range {
                key: "created_at".to_string(),
                lt: Some(cutoff as f64),
                gt: None,
            }],
            should: vec![],
        };

        let deleted = self.db.delete_by_filter(&self.collection, filter).await?;

        if deleted > 0 {
            debug!(deleted = deleted, "cleaned up old mid-term entries");
        }

        Ok(deleted)
    }
}

fn session_scope_filter(session_key: &SessionKey) -> SearchFilter {
    SearchFilter {
        must: vec![
            FilterCondition::Match {
                key: "guild_id".to_string(),
                value: json!(session_key.guild_id.map(|g| g.to_string())),
            },
            FilterCondition::Match {
                key: "channel_id".to_string(),
                value: json!(session_key.channel_id.to_string()),
            },
            FilterCondition::Match {
                key: "kind".to_string(),
                value: json!(session_kind_value(&session_key.kind)),
            },
        ],
        should: vec![],
    }
}

fn session_kind_value(kind: &SessionKind) -> &'static str {
    match kind {
        SessionKind::GuildChannel => "guild",
        SessionKind::Thread => "thread",
        SessionKind::DirectMessage => "dm",
    }
}
