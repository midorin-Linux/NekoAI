use crate::agent::memory::ConversationMemory;
use crate::agent::tools::tools::{execute_tool_call, tool_definitions};
use crate::models::message::MessageRole;
use crate::services::openai::OpenAiService;

use anyhow::{anyhow, Result};
use async_openai::types::chat::{
    ChatCompletionMessageToolCalls, ChatCompletionRequestAssistantMessageArgs,
};
use serenity::all::Context as SerenityContext;

pub struct ChatService {
    openai: OpenAiService,
    system_prompt: String,
    tool_system_prompt: String,
}

impl ChatService {
    pub fn new(openai: OpenAiService, system_prompt: String) -> Self {
        const TOOL_SYSTEM_PROMPT: &str = "You are helpful discord assistant";
        Self {
            openai,
            tool_system_prompt: TOOL_SYSTEM_PROMPT.to_string(),
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
        use_tools: bool,
        ctx: Option<&SerenityContext>,
    ) -> Result<String> {
        tracing::debug!("Chat with history: {}", user_message);

        let system_prompt = if use_tools {
            &self.tool_system_prompt
        } else {
            &self.system_prompt
        };

        let mut messages = vec![
            self.openai.create_system_message(system_prompt)?,
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

        if use_tools {
            let tool_ctx = ctx.ok_or_else(|| anyhow!("Tools require Discord context"))?;
            let tools = tool_definitions()?;
            let (assistant_content, tool_calls) = self
                .openai
                .create_chat_completion_with_tools(messages.clone(), tools, None)
                .await?;

            if let Some(tool_calls) = tool_calls {
                let mut assistant_message = ChatCompletionRequestAssistantMessageArgs::default();
                if !assistant_content.is_empty() {
                    assistant_message.content(assistant_content);
                }
                assistant_message.tool_calls(tool_calls.clone());
                messages.push(assistant_message.build()?.into());

                for tool_call in tool_calls {
                    match tool_call {
                        ChatCompletionMessageToolCalls::Function(call) => {
                            let output =
                                execute_tool_call(tool_ctx, &call.function.name, &call.function.arguments).await;
                            messages.push(
                                self.openai
                                    .create_tool_message(&call.id, &output)?,
                            );
                        }
                        ChatCompletionMessageToolCalls::Custom(call) => {
                            let output = format!("custom tool not supported: {}", call.custom_tool.name);
                            messages.push(
                                self.openai
                                    .create_tool_message(&call.id, &output)?,
                            );
                        }
                    }
                }

                let response = self.openai.create_chat_completion(messages).await?;
                tracing::debug!("Chat response with tools: {}", response);
                return Ok(response);
            }

            tracing::debug!("Chat response without tool calls: {}", assistant_content);
            return Ok(assistant_content);
        }

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

    pub fn update_tool_system_prompt(&mut self, new_prompt: String) {
        self.tool_system_prompt = new_prompt;
    }

    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }
}
