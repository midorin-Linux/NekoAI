use crate::shared::config::{Model, Provider};

use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait AIClient: Sized {
    async fn new(api_key: String, provider: Provider, model: Model) -> Result<Self>;
    async fn generate(&self, conversation: String) -> Result<String>;
}
