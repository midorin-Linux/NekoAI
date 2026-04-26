use std::fmt;

use anyhow::Result;
use config::{Config as ConfigBuilder, ConfigError, File};
use serde::Deserialize;
use tracing::{debug, info};

#[derive(Debug, Clone, Deserialize)]
pub struct Discord {
    pub token: SecretKey,
    pub guild_id: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ChatPlatform {
    #[default]
    Discord,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Parameters {
    pub max_token: u64,
    pub temperature: f64,
    pub top_p: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LanguageModel {
    pub provider_base_url: String,
    pub api_key: SecretKey,
    pub model_name: String,
    pub parameters: Parameters,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingModel {
    pub provider_base_url: String,
    pub api_key: SecretKey,
    pub model_name: String,
    pub dimension: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Provider {
    pub language_model: LanguageModel,
    pub embedding_model: EmbeddingModel,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VectorDb {
    #[serde(default = "default_qdrant_url")]
    pub url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_mid_term_collection")]
    pub mid_term_collection: String,
    #[serde(default = "default_long_term_collection")]
    pub long_term_collection: String,
}

impl Default for VectorDb {
    fn default() -> Self {
        Self {
            url: default_qdrant_url(),
            api_key: None,
            mid_term_collection: default_mid_term_collection(),
            long_term_collection: default_long_term_collection(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Memory {
    #[serde(default)]
    pub vector_db: VectorDb,
    #[serde(default = "default_short_term_max_entries")]
    pub short_term_max_entries: usize,
    #[serde(default = "default_mid_term_top_k")]
    pub mid_term_top_k: usize,
    #[serde(default = "default_long_term_top_k")]
    pub long_term_top_k: usize,
    #[serde(default = "default_mid_term_retention_days")]
    pub mid_term_retention_days: u32,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            vector_db: VectorDb::default(),
            short_term_max_entries: default_short_term_max_entries(),
            mid_term_top_k: default_mid_term_top_k(),
            long_term_top_k: default_long_term_top_k(),
            mid_term_retention_days: default_mid_term_retention_days(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub chat_platform: ChatPlatform,
    pub discord: Discord,
    pub provider: Provider,
    #[serde(default)]
    pub memory: Memory,
}

fn default_qdrant_url() -> String {
    "http://localhost:6334".to_string()
}

fn default_mid_term_collection() -> String {
    "mid_term".to_string()
}

fn default_long_term_collection() -> String {
    "long_term".to_string()
}

const fn default_short_term_max_entries() -> usize {
    20
}

const fn default_mid_term_top_k() -> usize {
    3
}

const fn default_long_term_top_k() -> usize {
    5
}

const fn default_mid_term_retention_days() -> u32 {
    30
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

#[derive(Clone, Deserialize)]
pub struct SecretKey(String);

impl SecretKey {
    pub fn expose(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for SecretKey {
    fn as_ref(&self) -> &str {
        self.expose()
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let visible_length = 4;
        let masked = {
            let length = self.0.chars().count();
            let start = length.saturating_sub(visible_length);
            let extracted: String = self.0.chars().skip(start).collect();
            format!("{:*>20}", &extracted)
        };
        f.debug_tuple("SecretKey").field(&masked).finish()
    }
}
