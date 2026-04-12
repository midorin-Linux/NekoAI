use anyhow::Result;
use config::{Config as ConfigBuilder, ConfigError, File};
use serde::Deserialize;
use tracing::{debug, info};

#[derive(Debug, Clone, Deserialize)]
pub struct Discord {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LanguageModel {
    pub provider_base_url: String,
    pub api_key: String,
    pub model_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingModel {
    pub provider_base_url: String,
    pub api_key: String,
    pub model_name: String,
    pub dimension: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Provider {
    pub language_model: LanguageModel,
    pub embedding_model: EmbeddingModel,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub discord: Discord,
    pub provider: Provider,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        info!("loading configuration file");
        let config = ConfigBuilder::builder()
            .add_source(
                File::with_name(".config/config.json")
                    .format(config::FileFormat::Json)
                    .required(true),
            )
            .build()?;

        debug!("configuration source parsed");

        let parsed = config.try_deserialize();

        if parsed.is_ok() {
            info!("configuration deserialized successfully");
        }

        parsed
    }
}
