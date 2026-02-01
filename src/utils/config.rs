use anyhow::Result;
use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use serde::Deserialize;

fn default_log_level() -> String { "info".to_string() }

fn default_max_token() -> u32 { 65536 }

fn default_temperature() -> f32 { 0.8 }

fn default_top_p() -> f32 { 0.95 }

fn default_frequency_penalty() -> f32 { 1.1 }

fn default_presence_penalty() -> f32 { 1.0 }

#[derive(Debug, Clone, Deserialize)]
pub struct Provider {
    pub api_key: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Model {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Sampling {
    #[serde(default = "default_max_token")]
    pub max_token: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    #[serde(default = "default_frequency_penalty")]
    pub frequency_penalty: f32,
    #[serde(default = "default_presence_penalty")]
    pub presence_penalty: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(rename = "token")]
    pub discord_token: String,

    #[serde(rename = "allowed_user_id")]
    pub allowed_user_id: Option<u64>,
    
    #[serde(rename = "target_guild_id")]
    pub target_guild_id: u64,

    #[serde(rename = "log_level", default = "default_log_level")]
    pub log_level: String,

    pub provider: Provider,
    pub model: Model,
    pub sampling: Sampling,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let config = ConfigBuilder::builder()
            .add_source(
                File::with_name("config/model.toml")
                    .required(true),
            )
            .add_source(
                File::with_name(".env")
                    .format(config::FileFormat::Ini)
                    .required(true),
            )
            .add_source(Environment::default().separator("__"))
            .build()?;

        config.try_deserialize()
    }
}