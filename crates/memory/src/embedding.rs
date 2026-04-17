use rig::{client::EmbeddingsClient as _, embeddings::EmbeddingModel as _, providers::openai};
use tracing::warn;

pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> Vec<f32>;
    fn dimension(&self) -> usize;
}

pub struct OpenAICompatibleEmbedder {
    model: openai::EmbeddingModel,
    dim: usize,
    fallback: MockEmbedder,
}

impl OpenAICompatibleEmbedder {
    pub fn new(
        base_url: &str,
        api_key: &str,
        model_name: &str,
        dim: usize,
    ) -> anyhow::Result<Self> {
        let client = openai::Client::builder()
            .api_key(api_key)
            .base_url(base_url)
            .build()?;

        let model = client.embedding_model_with_ndims(model_name, dim);

        Ok(Self {
            model,
            dim,
            fallback: MockEmbedder::new(dim),
        })
    }

    fn run_async<T, E>(future: impl Future<Output = Result<T, E>>) -> anyhow::Result<T>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let result = if tokio::runtime::Handle::try_current().is_ok() {
            tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
        } else {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(future)
        };

        result.map_err(Into::into)
    }
}

impl Embedder for OpenAICompatibleEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        match Self::run_async(self.model.embed_text(text)) {
            Ok(embedding) => embedding
                .vec
                .into_iter()
                .map(|value| value as f32)
                .collect(),
            Err(error) => {
                warn!(error = %error, "failed to embed text, falling back to mock embedder");
                self.fallback.embed(text)
            }
        }
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

pub struct MockEmbedder {
    dim: usize,
}

impl MockEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

impl Embedder for MockEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        let mut rng = RandSimple(stable_seed(text));
        (0 .. self.dim)
            .map(|_| rng.next_f32() * 2.0 - 1.0)
            .collect()
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

fn stable_seed(text: &str) -> u64 {
    let mut hash = 1469598103934665603u64;
    for &byte in text.as_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

struct RandSimple(u64);

impl RandSimple {
    fn next_f32(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        (self.0 >> 16) as f32 / 65536.0
    }
}
