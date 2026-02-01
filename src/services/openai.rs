use crate::utils::config::Sampling;

use anyhow::{anyhow, Result};
use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionMessageToolCalls, ChatCompletionRequestAssistantMessageArgs,
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestToolMessageArgs, ChatCompletionRequestUserMessageArgs,
        ChatCompletionToolChoiceOption, ChatCompletionTools, CreateChatCompletionRequestArgs,
    },
    Client as OpenAiClient,
};

pub struct OpenAiService {
    client: OpenAiClient<OpenAIConfig>,
    model: String,
    sampling: Sampling,
}

impl OpenAiService {
    pub fn new(api_key: &str, base_url: &str, model: &str, sampling: Sampling) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);
        let client = OpenAiClient::with_config(config);

        Self {
            client,
            model: model.to_string(),
            sampling
        }
    }

    pub async fn create_chat_completion(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<String> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages)
            .max_tokens(self.sampling.max_token)
            .temperature(self.sampling.temperature)
            .top_p(self.sampling.top_p)
            .frequency_penalty(self.sampling.frequency_penalty)
            .presence_penalty(self.sampling.presence_penalty)
            .build()?;

        let response = self.client.chat().create(request).await?;
        let choice = response
            .choices
            .first()
            .ok_or_else(|| anyhow!("No response from OpenAI"))?;

        choice
            .message
            .content
            .clone()
            .ok_or_else(|| anyhow!("No response content from OpenAI"))
    }

    pub async fn create_chat_completion_with_tools(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        tools: Vec<ChatCompletionTools>,
        tool_choice: Option<ChatCompletionToolChoiceOption>,
    ) -> Result<(String, Option<Vec<ChatCompletionMessageToolCalls>>)> {
        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder
            .model(&self.model)
            .messages(messages)
            .max_tokens(self.sampling.max_token)
            .temperature(self.sampling.temperature)
            .top_p(self.sampling.top_p)
            .frequency_penalty(self.sampling.frequency_penalty)
            .presence_penalty(self.sampling.presence_penalty);

        if !tools.is_empty() {
            request_builder.tools(tools);

            if let Some(choice) = tool_choice {
                request_builder.tool_choice(choice);
            }
        }

        let request = request_builder.build()?;
        let response = self.client.chat().create(request).await?;

        let choice = response
            .choices
            .first()
            .ok_or_else(|| anyhow!("No response from OpenAI"))?;

        if let Some(tool_calls) = &choice.message.tool_calls {
            let content = choice.message.content.clone().unwrap_or_default();
            return Ok((content, Some(tool_calls.clone())));
        }

        let content = choice
            .message
            .content
            .clone()
            .ok_or_else(|| anyhow!("No response content from OpenAI"))?;

        Ok((content, None))
    }

    pub fn create_system_message(&self, content: &str) -> Result<ChatCompletionRequestMessage> {
        Ok(ChatCompletionRequestSystemMessageArgs::default()
            .content(content)
            .build()?
            .into())
    }

    pub fn create_user_message(&self, content: &str) -> Result<ChatCompletionRequestMessage> {
        Ok(ChatCompletionRequestUserMessageArgs::default()
            .content(content)
            .build()?
            .into())
    }

    pub fn create_tool_message(
        &self,
        tool_call_id: &str,
        content: &str,
    ) -> Result<ChatCompletionRequestMessage> {
        Ok(ChatCompletionRequestToolMessageArgs::default()
            .content(content)
            .tool_call_id(tool_call_id)
            .build()?
            .into())
    }

    pub fn create_assistant_message(&self, content: &str) -> Result<ChatCompletionRequestMessage> {
        Ok(ChatCompletionRequestAssistantMessageArgs::default()
            .content(content)
            .build()?
            .into())
    }
}
