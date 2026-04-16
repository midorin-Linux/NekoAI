use anyhow::Result;
use nekoai_agent::runtime::AgentRuntime;
use nekoai_config::loader::{ChatPlatform, Config};
use nekoai_discord::client::DiscordClient;
use tracing::info;

const DISCORD_GUILD_ID: u64 = 1233632516750184489;

pub enum ChatClient {
    Discord(DiscordClient),
}

impl ChatClient {
    pub async fn initialize(config: &Config, runtime: AgentRuntime) -> Result<Self> {
        match &config.chat_platform {
            ChatPlatform::Discord => {
                info!("initializing discord chat client");
                let client =
                    DiscordClient::new(config.discord.token.clone(), DISCORD_GUILD_ID, runtime)
                        .await?;

                Ok(Self::Discord(client))
            }
        }
    }

    pub fn platform_name(&self) -> &'static str {
        match self {
            Self::Discord(_) => "discord",
        }
    }

    pub async fn run(self) -> Result<()> {
        match self {
            Self::Discord(client) => client.run().await,
        }
    }
}
