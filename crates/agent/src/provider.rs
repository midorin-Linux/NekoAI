use rig::{
    agent::AgentBuilder, client::CompletionClient,
    providers::openai::responses_api::ResponsesCompletionModel,
};

pub struct OpenAICompatibleAdapter {
    client: rig::providers::openai::Client,
}

impl OpenAICompatibleAdapter {
    pub fn new(client: rig::providers::openai::Client) -> Self {
        Self { client }
    }

    pub fn build_agent(&self, model: &str) -> AgentBuilder<ResponsesCompletionModel> {
        self.client.agent(model)
    }

    pub fn provider_name(&self) -> &str {
        "openai-compatible"
    }
}
