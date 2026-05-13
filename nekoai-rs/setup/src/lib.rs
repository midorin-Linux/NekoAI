pub mod cli_fallback;
pub mod config_writer;
pub mod wizard;

use anyhow::Result;
use nekoai_config::loader::Config;
use tracing::info;

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
    std::env::var("DISCORD_AGENT_TOKEN").is_ok()
}

/// Build a partial Config from the DISCORD_AGENT_TOKEN environment variable.
/// Other fields will use sensible defaults.
pub fn config_from_env() -> Option<Config> {
    let token = std::env::var("DISCORD_AGENT_TOKEN").ok()?;
    Some(cli_fallback::make_config(
        &token, "", "", "", "", 0, false, false,
    ))
}

/// Check if a configuration file already exists at the expected path.
pub fn config_exists() -> bool {
    std::fs::exists(".config/config.json").unwrap_or(false)
}
