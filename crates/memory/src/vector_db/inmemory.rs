use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;

use super::{
    FilterCondition, SearchFilter, SearchRequest, SearchResult, UpsertRequest, VectorDbClient,
};

pub struct InMemoryVectorDb {
    collections: Arc<std::sync::RwLock<HashMap<String, Vec<Point>>>>,
}

struct Point {
    id: String,
    vector: Vec<f32>,
    payload: HashMap<String, serde_json::Value>,
}

impl InMemoryVectorDb {
    pub fn new() -> Self {
        Self {
            collections: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryVectorDb {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorDbClient for InMemoryVectorDb {
    async fn upsert(&self, req: UpsertRequest<'_>) -> anyhow::Result<()> {
        let mut collections = self.collections.write().unwrap();
        let points = collections.entry(req.collection.to_string()).or_default();

        if let Some(existing) = points.iter_mut().find(|p| p.id == req.id) {
            existing.vector = req.vector.clone();
            existing.payload = req.payload.clone();
        } else {
            points.push(Point {
                id: req.id.to_string(),
                vector: req.vector.clone(),
                payload: req.payload.clone(),
            });
        }

        Ok(())
    }

    async fn search(&self, req: SearchRequest<'_>) -> anyhow::Result<Vec<SearchResult>> {
        let collections = self.collections.read().unwrap();
        let points: Option<&Vec<Point>> = collections.get(req.collection);
        let points = match points {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };

        let mut results: Vec<SearchResult> = points
            .iter()
            .filter(|p| {
                req.filter
                    .as_ref()
                    .is_none_or(|filter| matches_filter(&p.payload, filter))
            })
            .map(|p| {
                let score = cosine_similarity(&req.vector, &p.vector);
                SearchResult {
                    id: p.id.clone(),
                    score,
                    payload: p.payload.clone(),
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(req.top_k);

        Ok(results)
    }

    async fn delete(&self, collection: &str, id: &str) -> anyhow::Result<()> {
        let mut collections = self.collections.write().unwrap();
        if let Some(points) = collections.get_mut(collection) {
            points.retain(|p| p.id != id);
        }
        Ok(())
    }

    async fn delete_by_filter(
        &self,
        collection: &str,
        filter: SearchFilter,
    ) -> anyhow::Result<u64> {
        let mut collections = self.collections.write().unwrap();

        let Some(points) = collections.get_mut(collection) else {
            return Ok(0);
        };

        let before = points.len();
        points.retain(|p| !matches_filter(&p.payload, &filter));
        let deleted = (before - points.len()) as u64;

        Ok(deleted)
    }

    async fn ensure_collection(&self, name: &str, _dim: usize) -> anyhow::Result<()> {
        let mut collections = self.collections.write().unwrap();
        collections.entry(name.to_string()).or_default();
        Ok(())
    }
}

fn matches_filter(payload: &HashMap<String, serde_json::Value>, filter: &SearchFilter) -> bool {
    let must_ok = filter
        .must
        .iter()
        .all(|condition| matches_condition(payload, condition));

    let should_ok = filter.should.is_empty()
        || filter
            .should
            .iter()
            .any(|condition| matches_condition(payload, condition));

    must_ok && should_ok
}

fn matches_condition(
    payload: &HashMap<String, serde_json::Value>,
    condition: &FilterCondition,
) -> bool {
    match condition {
        FilterCondition::Match { key, value } => {
            let Some(actual) = payload.get(key) else {
                return false;
            };

            value_equals(actual, value)
        }
        FilterCondition::Range { key, lt, gt } => {
            let Some(actual) = payload.get(key).and_then(value_as_f64) else {
                return false;
            };

            if let Some(upper) = lt
                && actual >= *upper
            {
                return false;
            }

            if let Some(lower) = gt
                && actual <= *lower
            {
                return false;
            }

            true
        }
    }
}

fn value_equals(actual: &serde_json::Value, expected: &serde_json::Value) -> bool {
    match (actual, expected) {
        (serde_json::Value::Array(_), serde_json::Value::Array(_)) => actual == expected,
        (serde_json::Value::Array(items), _) => {
            items.iter().any(|item| value_equals(item, expected))
        }
        (serde_json::Value::Number(_), serde_json::Value::Number(_)) => {
            value_as_f64(actual) == value_as_f64(expected)
        }
        _ => actual == expected,
    }
}

fn value_as_f64(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|v| v as f64))
        .or_else(|| value.as_u64().map(|v| v as f64))
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}
