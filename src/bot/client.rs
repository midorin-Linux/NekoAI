use crate::agent::{agent::Agent, prompts::load_system_prompt};
use crate::bot::{commands::commands, handler::Handler};
use crate::services::openai::OpenAiService;
use crate::utils::config::Config;
use std::sync::Arc;

use anyhow::Result;
use serenity::{
    prelude::*,
};

pub struct App {
    config: Config,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn run(&mut self) -> Result<()> {
        println!();

        let intents = GatewayIntents::all();

        let openai_client = OpenAiService::new(
            &self.config.openai_api_key,
            &self.config.openai_base_url,
            &self.config.openai_model,
        );

        let system_prompt = load_system_prompt();
        let agent = Arc::new(Agent::new(openai_client, system_prompt));

        let framework = commands::command_framework(self.config.target_guild_id, agent.clone()).await;

        let mut client = Client::builder(&self.config.discord_token, intents)
            .event_handler(Handler {
                allowed_user_id: self.config.allowed_user_id,
                agent,
            })
            .framework(framework)
            .await
            .map_err(|e| anyhow::anyhow!("Error creating client: {:?}", e))?;

        client
            .start()
            .await
            .map_err(|e| anyhow::anyhow!("Error starting client: {:?}", e))?;

        Ok(())
    }
}
