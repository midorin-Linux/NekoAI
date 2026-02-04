use crate::models::user::UserInfo;
use chrono::{DateTime, Local};
use serenity::all::{ChannelId, Context as SerenityContext, GuildId, Message as DiscordMessage, UserId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
    pub user_id: UserId,
    pub user_info: UserInfo,
    pub timestamp: DateTime<Local>,
    pub message_reference: Option<MessageReference>,
}

#[derive(Debug, Clone)]
pub struct MessageReference {
    pub message_id: serenity::all::MessageId,
    pub author_name: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct GuildContext {
    pub guild_id: GuildId,
    pub guild_name: String,
    pub member_count: u64,
    pub channel_name: Option<String>,
}

pub struct AgentContext {
    serenity_ctx: SerenityContext,
    conversation: ConversationContext,
    guild: Option<GuildContext>,
    metadata: Arc<RwLock<HashMap<String, String>>>,
}

impl AgentContext {
    pub async fn new(
        serenity_ctx: SerenityContext,
        discord_message: &DiscordMessage,
    ) -> anyhow::Result<Self> {
        let user_info = UserInfo::from_discord_user(&discord_message.author);
        
        let guild = if let Some(guild_id) = discord_message.guild_id {
            if let Some(guild) = guild_id.to_guild_cached(&serenity_ctx) {
                let channel_name = discord_message
                    .channel_id
                    .name(&serenity_ctx)
                    .await
                    .ok();
                
                Some(GuildContext {
                    guild_id,
                    guild_name: guild.name.clone(),
                    member_count: guild.member_count,
                    channel_name,
                })
            } else {
                None
            }
        } else {
            None
        };

        let message_reference = discord_message.referenced_message.as_ref().map(|msg| {
            MessageReference {
                message_id: msg.id,
                author_name: msg.author.name.clone(),
                content: msg.content.clone(),
            }
        });

        let conversation = ConversationContext {
            channel_id: discord_message.channel_id,
            guild_id: discord_message.guild_id,
            user_id: discord_message.author.id,
            user_info,
            timestamp: Local::now(),
            message_reference,
        };

        Ok(Self {
            serenity_ctx,
            conversation,
            guild,
            metadata: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn serenity_context(&self) -> &SerenityContext {
        &self.serenity_ctx
    }

    pub fn conversation(&self) -> &ConversationContext {
        &self.conversation
    }

    pub fn guild(&self) -> Option<&GuildContext> {
        self.guild.as_ref()
    }

    pub async fn set_metadata(&self, key: String, value: String) {
        let mut metadata = self.metadata.write().await;
        metadata.insert(key, value);
    }

    pub async fn get_metadata(&self, key: &str) -> Option<String> {
        let metadata = self.metadata.read().await;
        metadata.get(key).cloned()
    }

    pub fn is_dm(&self) -> bool {
        self.conversation.guild_id.is_none()
    }

    pub fn format_context_for_prompt(&self) -> String {
        let mut context_parts = Vec::new();

        context_parts.push(format!(
            "現在時刻: {}",
            self.conversation.timestamp.format("%Y-%m-%d %H:%M:%S")
        ));

        if let Some(guild) = &self.guild {
            context_parts.push(format!("サーバー: {}", guild.guild_name));
            if let Some(channel_name) = &guild.channel_name {
                context_parts.push(format!("チャンネル: #{}" , channel_name));
            }
            context_parts.push(format!("メンバー数: {}人", guild.member_count));
        } else {
            context_parts.push("DMでの会話".to_string());
        }

        context_parts.push(format!(
            "ユーザー: {} (ID: {})",
            self.conversation.user_info.username,
            self.conversation.user_id
        ));

        if let Some(nick) = &self.conversation.user_info.nickname {
            context_parts.push(format!("ニックネーム: {}", nick));
        }

        if let Some(ref_msg) = &self.conversation.message_reference {
            context_parts.push(format!(
                "返信先メッセージ: {}さん「{}」",
                ref_msg.author_name,
                if ref_msg.content.len() > 100 {
                    format!("{}...", &ref_msg.content[..100])
                } else {
                    ref_msg.content.clone()
                }
            ));
        }

        context_parts.join("\n")
    }

    pub fn get_channel_id(&self) -> ChannelId {
        self.conversation.channel_id
    }

    pub fn get_user_id(&self) -> UserId {
        self.conversation.user_id
    }
}

#[derive(Debug, Clone, Default)]
pub struct ContextBuilder {
    metadata: HashMap<String, String>,
}

impl ContextBuilder {
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn build(self) -> HashMap<String, String> {
        self.metadata
    }
}