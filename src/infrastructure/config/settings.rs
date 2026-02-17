use anyhow::Result;
use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use serde::Deserialize;

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub ai_provider_token: String,

    pub discord_token: String,

    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let config = ConfigBuilder::builder()
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
