use anyhow::Result;
use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use serde::Deserialize;

fn default_log_level() -> String {
    "info".to_string()
}

fn default_max_agent_turns() -> usize {
    10
}

fn default_midterm_expiry_days() -> u64 {
    7
}

fn default_midterm_search_limit() -> usize {
    3
}

fn default_longterm_search_limit() -> usize {
    5
}

fn default_min_search_score() -> f32 {
    0.5
}

fn default_messages_per_minute() -> u32 {
    10
}

fn default_cooldown_seconds() -> u64 {
    5
}

#[derive(Debug, Clone, Deserialize)]
pub struct NLP {
    pub api_url: String,
    pub model_name: String,
    pub max_short_term_messages: usize,

    #[serde(default = "default_max_agent_turns")]
    pub max_agent_turns: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Embedding {
    pub api_url: String,
    pub model_name: String,
    pub dimension: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_midterm_expiry_days")]
    pub midterm_expiry_days: u64,

    #[serde(default = "default_midterm_search_limit")]
    pub midterm_search_limit: usize,

    #[serde(default = "default_longterm_search_limit")]
    pub longterm_search_limit: usize,

    #[serde(default = "default_min_search_score")]
    pub min_search_score: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            midterm_expiry_days: default_midterm_expiry_days(),
            midterm_search_limit: default_midterm_search_limit(),
            longterm_search_limit: default_longterm_search_limit(),
            min_search_score: default_min_search_score(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_messages_per_minute")]
    pub messages_per_minute: u32,

    #[serde(default = "default_cooldown_seconds")]
    pub cooldown_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            messages_per_minute: default_messages_per_minute(),
            cooldown_seconds: default_cooldown_seconds(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub nlp_token: String,
    pub embed_token: String,
    pub discord_token: String,
    pub guild_id: u64,
    pub qdrant_url: String,

    #[serde(default = "default_log_level")]
    pub log_level: String,

    pub nlp: NLP,
    pub embedding: Embedding,

    #[serde(default)]
    pub memory: MemoryConfig,

    #[serde(default)]
    pub rate_limit: RateLimitConfig,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let config = ConfigBuilder::builder()
            .add_source(
                File::with_name(".env")
                    .format(config::FileFormat::Ini)
                    .required(true),
            )
            .add_source(
                File::with_name("config/settings.toml")
                    .format(config::FileFormat::Toml)
                    .required(true),
            )
            .add_source(Environment::default().separator("__"))
            .build()?;

        config.try_deserialize()
    }
}
