use nekoai_config::loader::{
    ChatPlatform, Config, ConversationModel, Discord, EmbeddingModel, Memory, Parameters, Provider,
    SecretKey, SummarizerModel, ToolPermissions, VectorDb,
};

/// Build a Config from explicitly provided CLI arguments.
/// Missing optional fields use sensible defaults.
pub fn make_config(token: &str, provider: &str, model: &str) -> Config {
    let (provider_base_url, model_name) = resolve_provider(provider, model);

    Config {
        chat_platform: ChatPlatform::Discord,
        discord: Discord {
            token: SecretKey::new(token.to_owned()),
            guild_id: 0,
        },
        provider: Provider {
            conversation_model: ConversationModel {
                provider_base_url: provider_base_url.clone(),
                api_key: SecretKey::new("".to_owned()),
                model_name,
                parameters: Parameters {
                    max_token: 262144,
                    temperature: 1.0,
                    top_p: 0.95,
                },
            },
            summarizer_model: SummarizerModel {
                provider_base_url: provider_base_url.clone(),
                api_key: SecretKey::new("".to_owned()),
                model_name: "gpt-4o-mini".to_owned(),
                parameters: Parameters {
                    max_token: 262144,
                    temperature: 1.0,
                    top_p: 0.95,
                },
            },
            embedding_model: EmbeddingModel {
                provider_base_url,
                api_key: SecretKey::new("".to_owned()),
                model_name: "text-embedding-3-small".to_owned(),
                dimension: 1536,
            },
        },
        memory: Memory {
            vector_db: VectorDb::default(),
            short_term_max_entries: 20,
            mid_term_top_k: 3,
            long_term_top_k: 5,
            mid_term_retention_days: 30,
        },
        tools: ToolPermissions {
            web_search: false,
            code_exec: false,
        },
    }
}

/// Resolve the provider base URL and a default model name.
/// Currently only supports "custom" provider (others are treated as custom too).
fn resolve_provider(_provider: &str, model: &str) -> (String, String) {
    let model = if model.is_empty() { "gpt-4o" } else { model };
    let base_url = "https://api.openai.com/v1";
    (base_url.to_owned(), model.to_owned())
}
