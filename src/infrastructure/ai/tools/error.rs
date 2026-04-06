use std::fmt;
use std::sync::Arc;

use serenity::all::{Cache, GuildId, Http, Permissions, UserId};

use crate::shared::permission_checker::PermissionChecker;

/// Discord ツールエラーの種別
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscordToolErrorKind {
    /// 権限不足
    PermissionDenied,
    /// 対象が見つからない
    NotFound,
    /// 引数が不正
    InvalidArgument,
    /// レート制限
    RateLimited,
    /// Discord API エラー
    ApiError,
    /// 内部エラー
    Internal,
}

/// Discord ツール共通のエラー型
#[derive(Debug)]
pub struct DiscordToolError {
    pub tool_name: String,
    pub kind: DiscordToolErrorKind,
    pub message: String,
}

impl DiscordToolError {
    pub fn new(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            kind: DiscordToolErrorKind::Internal,
            message: message.into(),
        }
    }

    pub fn with_kind(
        tool_name: impl Into<String>,
        kind: DiscordToolErrorKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            kind,
            message: message.into(),
        }
    }

    pub fn permission_denied(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::with_kind(tool_name, DiscordToolErrorKind::PermissionDenied, message)
    }

    pub fn not_found(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::with_kind(tool_name, DiscordToolErrorKind::NotFound, message)
    }

    pub fn invalid_argument(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::with_kind(tool_name, DiscordToolErrorKind::InvalidArgument, message)
    }

    pub fn rate_limited(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::with_kind(tool_name, DiscordToolErrorKind::RateLimited, message)
    }

    pub fn api_error(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::with_kind(tool_name, DiscordToolErrorKind::ApiError, message)
    }
}

impl fmt::Display for DiscordToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind_label = match self.kind {
            DiscordToolErrorKind::PermissionDenied => "PERMISSION_DENIED",
            DiscordToolErrorKind::NotFound => "NOT_FOUND",
            DiscordToolErrorKind::InvalidArgument => "INVALID_ARGUMENT",
            DiscordToolErrorKind::RateLimited => "RATE_LIMITED",
            DiscordToolErrorKind::ApiError => "API_ERROR",
            DiscordToolErrorKind::Internal => "INTERNAL",
        };
        write!(f, "[{}:{}] {}", self.tool_name, kind_label, self.message)
    }
}

impl std::error::Error for DiscordToolError {}

/// ツール実行前にリクエストユーザーの権限を検証する共通ヘルパー。
///
/// 指定された Discord 権限を持たないユーザーからの操作を拒否する。
/// `ADMINISTRATOR` を持つユーザーは全権限を持つものとして扱われる。
pub async fn require_user_permission(
    cache: &Arc<Cache>,
    http: &Arc<Http>,
    guild_id: GuildId,
    user_id: UserId,
    required: Permissions,
    tool_name: &str,
) -> Result<(), DiscordToolError> {
    PermissionChecker::user_has_permissions(cache, http, guild_id, user_id, required)
        .await
        .map_err(|e| DiscordToolError::new(tool_name, e))?
        .then_some(())
        .ok_or_else(|| {
            DiscordToolError::permission_denied(
                tool_name,
                format!(
                    "The requesting user does not have the required permission ({:?}) to use this tool.",
                    required
                ),
            )
        })
}
