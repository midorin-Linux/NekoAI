use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("AI generation error: {0}")]
    AIGeneration(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Store error: {0}")]
    Store(String),

    #[error("Discord error: {0}")]
    Discord(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Conversation not found: channel {0}")]
    ConversationNotFound(u64),

    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Tool execution error: {tool_name} - {message}")]
    ToolExecution { tool_name: String, message: String },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn user_facing_message(&self) -> &str {
        match self {
            AppError::AIGeneration(_) => "AI応答の生成に失敗しました。",
            AppError::Embedding(_) => "テキストの処理に失敗しました。",
            AppError::Store(_) => "記憶の検索に失敗しました。",
            AppError::Discord(_) => "Discordとの通信に失敗しました。",
            AppError::Config(_) => "設定の読み込みに失敗しました。",
            AppError::ConversationNotFound(_) => "会話が見つかりませんでした。",
            AppError::PermissionDenied { .. } => "権限がありません。",
            AppError::RateLimited(_) => {
                "リクエストが多すぎます。少し待ってから再試行してください。"
            }
            AppError::ToolExecution { .. } => "ツールの実行に失敗しました。",
            AppError::Validation(_) => "入力が不正です。",
            AppError::Internal(_) => "予期しないエラーが発生しました。",
        }
    }
}
