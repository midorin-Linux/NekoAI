use anyhow::Result;
use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use serde::Deserialize;

fn default_log_level() -> String {
    "info".to_string()
}

fn default_openai_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_openai_model() -> String {
    "gpt-3.5-turbo".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(rename = "token")]
    pub discord_token: String,

    #[serde(rename = "openai_api_key")]
    pub openai_api_key: String,

    #[serde(rename = "openai_base_url", default = "default_openai_base_url")]
    pub openai_base_url: String,

    #[serde(rename = "openai_model", default = "default_openai_model")]
    pub openai_model: String,

    #[serde(rename = "allowed_user_id")]
    pub allowed_user_id: Option<u64>,
    
    #[serde(rename = "target_guild_id")]
    pub target_guild_id: u64,

    #[serde(rename = "log_level", default = "default_log_level")]
    pub log_level: String,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let config = ConfigBuilder::builder()
            .add_source(
                File::with_name(".env")
                    .format(config::FileFormat::Ini)
                    .required(false),
            )
            .add_source(Environment::default().separator("__"))
            .build()?;

        config.try_deserialize()
    }
}