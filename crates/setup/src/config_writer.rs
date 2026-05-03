use std::path::Path;

use anyhow::{Context, Result};
use nekoai_config::loader::Config;
use tracing::{info, warn};

const CONFIG_PATH: &str = ".config/config.json";

/// Save a Config to the config file path.
/// If the file already exists, merge the new config with the existing one
/// (existing values take priority).
pub fn save_config(config: &Config) -> Result<()> {
    let config_dir = Path::new(CONFIG_PATH).parent().unwrap();
    if !config_dir.exists() {
        std::fs::create_dir_all(config_dir).with_context(|| {
            format!(
                "failed to create config directory: {}",
                config_dir.display()
            )
        })?;
        info!(path = %config_dir.display(), "created config directory");
    }

    let final_config = if Path::new(CONFIG_PATH).exists() {
        info!("existing config file found, merging with new values");
        merge_with_existing(config)?
    } else {
        config.clone()
    };

    let json = serde_json::to_string_pretty(&final_config)
        .context("failed to serialize config to JSON")?;

    std::fs::write(CONFIG_PATH, &json)
        .with_context(|| format!("failed to write config file: {CONFIG_PATH}"))?;

    info!(path = CONFIG_PATH, "configuration saved successfully");
    Ok(())
}

/// Merge the wizard-generated config with an existing config file.
/// Existing values take priority over new values — this prevents accidentally
/// overwriting user-customized settings.
fn merge_with_existing(new_config: &Config) -> Result<Config> {
    let existing_content = std::fs::read_to_string(CONFIG_PATH)
        .with_context(|| format!("failed to read existing config: {CONFIG_PATH}"))?;

    let existing: Config = serde_json::from_str(&existing_content)
        .with_context(|| format!("failed to parse existing config: {CONFIG_PATH}"))?;

    let mut merged = new_config.clone();

    // Discord: keep existing token if it's not a placeholder
    if !is_placeholder(existing.discord.token.expose()) {
        merged.discord.token = existing.discord.token;
    }
    if existing.discord.guild_id != 0 {
        merged.discord.guild_id = existing.discord.guild_id;
    }

    // Provider (language model): keep existing API key if not placeholder
    if !is_placeholder(existing.provider.conversation_model.api_key.expose()) {
        merged.provider.conversation_model.api_key = existing.provider.conversation_model.api_key;
    }
    // Keep existing model name and base URL if they differ from defaults
    if !existing.provider.conversation_model.model_name.is_empty() {
        merged.provider.conversation_model.model_name =
            existing.provider.conversation_model.model_name.clone();
    }
    if !existing
        .provider
        .conversation_model
        .provider_base_url
        .is_empty()
    {
        merged.provider.conversation_model.provider_base_url =
            existing.provider.conversation_model.provider_base_url.clone();
    }

    // Provider (summarizer model): keep existing API key if not placeholder
    if !is_placeholder(existing.provider.summarizer_model.api_key.expose()) {
        merged.provider.summarizer_model.api_key = existing.provider.summarizer_model.api_key;
    }
    if !existing.provider.summarizer_model.model_name.is_empty() {
        merged.provider.summarizer_model.model_name =
            existing.provider.summarizer_model.model_name.clone();
    }
    if !existing
        .provider
        .summarizer_model
        .provider_base_url
        .is_empty()
    {
        merged.provider.summarizer_model.provider_base_url =
            existing.provider.summarizer_model.provider_base_url.clone();
    }
    if existing
        .provider
        .summarizer_model
        .parameters
        .max_token
        .abs_diff(262144)
        > 1
    {
        merged.provider.summarizer_model.parameters.max_token =
            existing.provider.summarizer_model.parameters.max_token;
    }
    if (existing.provider.summarizer_model.parameters.temperature - 1.0).abs() > 0.01 {
        merged.provider.summarizer_model.parameters.temperature =
            existing.provider.summarizer_model.parameters.temperature;
    }
    if (existing.provider.summarizer_model.parameters.top_p - 0.95).abs() > 0.01 {
        merged.provider.summarizer_model.parameters.top_p =
            existing.provider.summarizer_model.parameters.top_p;
    }

    // Provider (embedding model): keep existing API key if not placeholder
    if !is_placeholder(existing.provider.embedding_model.api_key.expose()) {
        merged.provider.embedding_model.api_key = existing.provider.embedding_model.api_key;
    }
    if !existing.provider.embedding_model.model_name.is_empty() {
        merged.provider.embedding_model.model_name =
            existing.provider.embedding_model.model_name.clone();
    }
    if !existing
        .provider
        .embedding_model
        .provider_base_url
        .is_empty()
    {
        merged.provider.embedding_model.provider_base_url =
            existing.provider.embedding_model.provider_base_url.clone();
    }
    if existing.provider.embedding_model.dimension != 0 {
        merged.provider.embedding_model.dimension = existing.provider.embedding_model.dimension;
    }

    // Parameters: keep existing if they differ from CLI defaults
    if existing
        .provider
        .conversation_model
        .parameters
        .max_token
        .abs_diff(262144)
        > 1
    {
        merged.provider.conversation_model.parameters.max_token =
            existing.provider.conversation_model.parameters.max_token;
    }
    if (existing.provider.conversation_model.parameters.temperature - 1.0).abs() > 0.01 {
        merged.provider.conversation_model.parameters.temperature =
            existing.provider.conversation_model.parameters.temperature;
    }
    if (existing.provider.conversation_model.parameters.top_p - 0.95).abs() > 0.01 {
        merged.provider.conversation_model.parameters.top_p =
            existing.provider.conversation_model.parameters.top_p;
    }

    // Memory settings: keep existing if they differ from defaults
    if existing.memory.short_term_max_entries != 20 {
        merged.memory.short_term_max_entries = existing.memory.short_term_max_entries;
    }
    if existing.memory.mid_term_top_k != 3 {
        merged.memory.mid_term_top_k = existing.memory.mid_term_top_k;
    }
    if existing.memory.long_term_top_k != 5 {
        merged.memory.long_term_top_k = existing.memory.long_term_top_k;
    }
    if existing.memory.mid_term_retention_days != 30 {
        merged.memory.mid_term_retention_days = existing.memory.mid_term_retention_days;
    }
    // Vector DB: keep existing if set
    if !existing.memory.vector_db.url.is_empty()
        && existing.memory.vector_db.url != "http://localhost:6334"
    {
        merged.memory.vector_db.url = existing.memory.vector_db.url.clone();
    }
    if existing.memory.vector_db.api_key.is_some() {
        merged.memory.vector_db.api_key = existing.memory.vector_db.api_key.clone();
    }

    // Tools: keep existing
    merged.tools = existing.tools.clone();

    warn!("existing config values were preserved where present");
    Ok(merged)
}

/// Check if a string looks like a placeholder (e.g. "YOUR_..." or empty).
fn is_placeholder(s: &str) -> bool {
    s.is_empty() || s.starts_with("YOUR_") || s.starts_with("sk-...") || s == "sk-ant-..."
}
