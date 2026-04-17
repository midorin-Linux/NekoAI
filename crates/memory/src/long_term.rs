use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use chrono::{DateTime, Utc};
use nekoai_domain::agent::session::{SessionKey, SessionKind};
use serde_json::json;
use tracing::{debug, info};
use uuid::Uuid;

use crate::{
    embedding::Embedder,
    store::MemoryEntry,
    vector_db::{FilterCondition, SearchFilter, VectorDbClient},
};

pub struct LongTermMemory {
    db: Arc<dyn VectorDbClient>,
    embedder: Arc<dyn Embedder>,
    collection: String,
}

impl LongTermMemory {
    pub fn new(
        db: Arc<dyn VectorDbClient>,
        embedder: Arc<dyn Embedder>,
        collection: String,
    ) -> Self {
        Self {
            db,
            embedder,
            collection,
        }
    }

    pub fn ensure_collection(&self, dim: usize) -> Result<()> {
        self.db.ensure_collection(&self.collection, dim)?;
        info!(collection = %self.collection, "long-term memory initialized");
        Ok(())
    }

    pub fn store(&self, session_key: &SessionKey, fact: String, tags: Vec<String>) -> Result<()> {
        let embedding = self.embedder.embed(&fact);
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        let mut payload = HashMap::new();
        payload.insert("content".to_string(), json!(fact));
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
        payload.insert("tags".to_string(), json!(tags));

        self.db.upsert(crate::vector_db::UpsertRequest {
            collection: &self.collection,
            id: &id,
            vector: embedding,
            payload,
        })?;

        debug!(id = %id, session = %session_key.channel_id, "stored long-term fact");
        Ok(())
    }

    pub fn search(
        &self,
        session_key: &SessionKey,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<MemoryEntry>> {
        let embedding = self.embedder.embed(query);

        let filter = session_scope_filter(session_key);

        let results = self.db.search(crate::vector_db::SearchRequest {
            collection: &self.collection,
            vector: embedding,
            filter: Some(filter),
            top_k,
        })?;

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

    pub fn delete(&self, id: &str) -> Result<()> {
        self.db.delete(&self.collection, id)?;
        debug!(id = %id, "deleted long-term fact");
        Ok(())
    }

    pub fn delete_by_channel(&self, channel_id: &str) -> Result<u64> {
        let filter = SearchFilter {
            must: vec![FilterCondition::Match {
                key: "channel_id".to_string(),
                value: json!(channel_id),
            }],
            should: vec![],
        };

        let deleted = self.db.delete_by_filter(&self.collection, filter)?;

        if deleted > 0 {
            debug!(
                channel_id = channel_id,
                deleted = deleted,
                "cleared long-term memories for channel"
            );
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
