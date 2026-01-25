use std::ops::Add;
use std::path::Path;

pub fn load_system_prompt() -> String {
    const DEFAULT_PROMPT: &str = "You are a helpful assistant.";
    const METADATA_PROMPT: &str = "# format of metadata\n<metadata>\nGuild: <guild_name> (<guild_id>)\nChannel: <category_name> > <channel_name> (<channel_id>)\nUser: <user_name> (<user_id>)\n</metadata>\n\n";
    let path = Path::new("prompts/system_prompt.txt");

    match std::fs::read_to_string(path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                DEFAULT_PROMPT.to_string()
            } else {
                trimmed.to_string().add(METADATA_PROMPT)
            }
        }
        Err(err) => {
            tracing::warn!(
                "Failed to read system prompt from {:?}: {:?}",
                path,
                err
            );
            DEFAULT_PROMPT.to_string()
        }
    }
}