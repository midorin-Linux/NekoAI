use rig::{
    agent::AgentBuilder, client::CompletionClient,
    providers::openrouter::completion::CompletionModel,
};

pub struct OpenRouterAdapter {
    client: rig::providers::openrouter::Client,
}

impl OpenRouterAdapter {
    pub fn new(client: rig::providers::openrouter::Client) -> Self {
        Self { client }
    }

    pub fn build_agent(&self, model: &str) -> AgentBuilder<CompletionModel> {
        self.client.agent(model)
    }

    pub fn provider_name(&self) -> &str {
        "openrouter"
    }
}
