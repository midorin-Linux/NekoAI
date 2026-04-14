use rig::{
    agent::AgentBuilder,
    client::CompletionClient,
    providers::openai::{self, completion::CompletionModel},
};

pub struct OpenAICompatibleAdapter {
    client: openai::CompletionsClient,
}

impl OpenAICompatibleAdapter {
    pub fn new(client: openai::CompletionsClient) -> Self {
        Self { client }
    }

    pub fn build_agent(&self, model: &str) -> AgentBuilder<CompletionModel> {
        self.client.agent(model)
    }

    pub fn provider_name(&self) -> &str {
        "openai-compatible"
    }
}
