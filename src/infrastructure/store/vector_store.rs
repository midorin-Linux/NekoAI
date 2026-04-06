use anyhow::Result;
use async_trait::async_trait;
use qdrant_client::{
    Payload, Qdrant,
    qdrant::{
        Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter, PointStruct,
        QueryPointsBuilder, Range, UpsertPointsBuilder, VectorParamsBuilder,
    },
};

use crate::{
    application::traits::long_term_store::LongTermStore,
    models::memory::{LongTermMemory, MidTermMemory},
};

const MIDTERM_COLLECTION_NAME: &str = "midterm_memory";
const LONGTERM_COLLECTION_NAME: &str = "longterm_memory";

pub struct VectorStore {
    qdrant_client: Qdrant,
    /// 検索結果の最低スコア閾値（これ未満のスコアの結果を除外する）
    min_search_score: f32,
}

impl VectorStore {
    pub async fn new(url: &str, dimension: u64, min_search_score: f32) -> Result<Self> {
        let client = Qdrant::from_url(url).build()?;

        if !client.collection_exists(MIDTERM_COLLECTION_NAME).await? {
            client
                .create_collection(
                    CreateCollectionBuilder::new(MIDTERM_COLLECTION_NAME)
                        .vectors_config(VectorParamsBuilder::new(dimension, Distance::Cosine)),
                )
                .await?;
        }

        if !client.collection_exists(LONGTERM_COLLECTION_NAME).await? {
            client
                .create_collection(
                    CreateCollectionBuilder::new(LONGTERM_COLLECTION_NAME)
                        .vectors_config(VectorParamsBuilder::new(dimension, Distance::Cosine)),
                )
                .await?;
        }

        Ok(Self {
            qdrant_client: client,
            min_search_score,
        })
    }
}

#[async_trait]
impl LongTermStore for VectorStore {
    async fn store_longterm(&self, memory: LongTermMemory, embedding: Vec<f32>) -> Result<()> {
        let payload_json = serde_json::to_value(&memory)?;
        let payload: Payload = Payload::try_from(payload_json)
            .map_err(|_| anyhow::anyhow!("Failed to convert memory to payload"))?;

        self.qdrant_client
            .upsert_points(
                UpsertPointsBuilder::new(
                    LONGTERM_COLLECTION_NAME,
                    vec![PointStruct::new(memory.id.clone(), embedding, payload)],
                )
                .wait(true),
            )
            .await?;

        Ok(())
    }

    async fn store_midterm(&self, memory: MidTermMemory, embedding: Vec<f32>) -> Result<()> {
        let payload_json = serde_json::to_value(&memory)?;
        let payload: Payload = Payload::try_from(payload_json)
            .map_err(|_| anyhow::anyhow!("Failed to convert memory to payload"))?;

        self.qdrant_client
            .upsert_points(
                UpsertPointsBuilder::new(
                    MIDTERM_COLLECTION_NAME,
                    vec![PointStruct::new(memory.id.clone(), embedding, payload)],
                )
                .wait(true),
            )
            .await?;

        Ok(())
    }

    async fn search_longterm(
        &self,
        embedding: Vec<f32>,
        user_id: u64,
        limit: u64,
    ) -> Result<Vec<LongTermMemory>> {
        let response = self
            .qdrant_client
            .query(
                QueryPointsBuilder::new(LONGTERM_COLLECTION_NAME)
                    .query(embedding)
                    .filter(Filter::must([Condition::matches(
                        "user_id",
                        user_id as i64,
                    )]))
                    .limit(limit)
                    .score_threshold(self.min_search_score)
                    .with_payload(true),
            )
            .await?;

        let mut memories = Vec::new();
        for point in response.result {
            let payload_value = serde_json::to_value(point.payload)?;
            let memory: LongTermMemory = serde_json::from_value(payload_value)?;
            memories.push(memory);
        }

        Ok(memories)
    }

    async fn search_midterm(
        &self,
        embedding: Vec<f32>,
        user_id: u64,
        limit: u64,
    ) -> Result<Vec<MidTermMemory>> {
        let response = self
            .qdrant_client
            .query(
                QueryPointsBuilder::new(MIDTERM_COLLECTION_NAME)
                    .query(embedding)
                    .filter(Filter::must([Condition::matches(
                        "user_id",
                        user_id as i64,
                    )]))
                    .limit(limit)
                    .score_threshold(self.min_search_score)
                    .with_payload(true),
            )
            .await?;

        let mut memories = Vec::new();
        for point in response.result {
            let payload_value = serde_json::to_value(point.payload)?;
            let memory: MidTermMemory = serde_json::from_value(payload_value)?;
            memories.push(memory);
        }

        Ok(memories)
    }

    async fn delete_expired_midterm(&self) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as f64;

        let filter = Filter::must([Condition::range(
            "expires_at",
            Range {
                lt: Some(now),
                ..Default::default()
            },
        )]);

        self.qdrant_client
            .delete_points(
                DeletePointsBuilder::new(MIDTERM_COLLECTION_NAME)
                    .points(filter)
                    .wait(true),
            )
            .await?;

        tracing::info!("Deleted expired midterm memories");
        Ok(())
    }
}
