use std::path::Path;

pub fn load_system_prompt() -> String {
    const DEFAULT_PROMPT: &str = "You are a helpful assistant.";
    let path = Path::new("prompts/system_prompt.txt");

    match std::fs::read_to_string(path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                DEFAULT_PROMPT.to_string()
            } else {
                trimmed.to_string()
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