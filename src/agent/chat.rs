use crate::agent::memory::ConversationMemory;
use crate::models::message::MessageRole;
use crate::services::openai::OpenAiService;

use anyhow::Result;
use async_openai::types::chat::ChatCompletionRequestAssistantMessageArgs;

pub struct ChatService {
    openai: OpenAiService,
    system_prompt: String,
}

impl ChatService {
    pub fn new(openai: OpenAiService, system_prompt: String) -> Self {
        Self {
            openai,
            system_prompt,
        }
    }

    pub async fn single_chat(&self, user_message: &str) -> Result<String> {
        tracing::debug!("Single chat request: {}", user_message);

        let messages = vec![
            self.openai.create_system_message(&self.system_prompt)?,
            self.openai.create_user_message(user_message)?,
        ];

        let response = self.openai.create_chat_completion(messages).await?;

        tracing::debug!("Single chat response: {}", response);
        Ok(response)
    }

    pub async fn chat_with_history(
        &self,
        user_message: &str,
        memory: &ConversationMemory,
    ) -> Result<String> {
        tracing::debug!("Chat with history: {}", user_message);

        let mut messages = vec![
            self.openai.create_system_message(&self.system_prompt)?,
        ];

        for msg in memory.get_messages() {
            let message = match msg.role {
                MessageRole::User => self.openai.create_user_message(&msg.content)?,
                MessageRole::Assistant => {
                    ChatCompletionRequestAssistantMessageArgs::default()
                        .content(msg.content.as_str())
                        .build()?
                        .into()
                }
                MessageRole::System => self.openai.create_system_message(&msg.content)?,
            };
            messages.push(message);
        }

        messages.push(self.openai.create_user_message(user_message)?);

        let response = self.openai.create_chat_completion(messages).await?;

        tracing::debug!("Chat response: {}", response);
        Ok(response)
    }

    #[allow(dead_code)]
    pub async fn streaming_chat(&self, _user_message: &str) -> Result<String> {
        // TODO: ストリーミングAPIの実装
        todo!("Streaming chat not implemented yet")
    }

    pub fn update_system_prompt(&mut self, new_prompt: String) {
        self.system_prompt = new_prompt;
    }

    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }
}