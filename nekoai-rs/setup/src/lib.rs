pub mod cli_fallback;
pub mod config_writer;
pub mod wizard;

use anyhow::Result;
use nekoai_config::loader::Config;
use tracing::info;

/// Environment variable names for configuration (preferred over CLI flags).
pub const ENV_DISCORD_TOKEN: &str = "DISCORD_AGENT_TOKEN";
pub const ENV_API_KEY: &str = "NEKOAI_API_KEY";
pub const ENV_PROVIDER: &str = "NEKOAI_PROVIDER";
pub const ENV_MODEL: &str = "NEKOAI_MODEL";
pub const ENV_BASE_URL: &str = "NEKOAI_BASE_URL";
pub const ENV_GUILD_ID: &str = "NEKOAI_GUILD_ID";

/// Run the interactive setup wizard (dialoguer-based).
/// This collects all necessary configuration from the user step by step,
/// saves the config file, and returns the generated Config.
pub async fn run_setup_wizard() -> Result<Config> {
    info!("starting interactive setup wizard");
    let config = wizard::run_wizard()?;
    config_writer::save_config(&config)?;
    info!("setup wizard completed successfully");
    Ok(config)
}

/// Check if the DISCORD_AGENT_TOKEN environment variable is set.
/// When set, the setup wizard can be skipped entirely.
pub fn has_env_token() -> bool {
    std::env::var(ENV_DISCORD_TOKEN).is_ok()
}

/// Build a Config from environment variables.
/// Supports: DISCORD_AGENT_TOKEN, NEKOAI_API_KEY, NEKOAI_PROVIDER,
/// NEKOAI_MODEL, NEKOAI_BASE_URL, NEKOAI_GUILD_ID.
/// Falls back to sensible defaults for missing values.
pub fn config_from_env() -> Option<Config> {
    let token = std::env::var(ENV_DISCORD_TOKEN).ok()?;
    let api_key = std::env::var(ENV_API_KEY).unwrap_or_default();
    let provider = std::env::var(ENV_PROVIDER).unwrap_or_default();
    let model = std::env::var(ENV_MODEL).unwrap_or_default();
    let base_url = std::env::var(ENV_BASE_URL).unwrap_or_default();
    let guild_id = std::env::var(ENV_GUILD_ID)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    Some(cli_fallback::make_config(
        &token, &api_key, &provider, &model, &base_url, guild_id, false,
    ))
}

/// Check if a configuration file already exists at the expected path.
pub fn config_exists() -> bool {
    std::fs::exists(".config/config.json").unwrap_or(false)
}
