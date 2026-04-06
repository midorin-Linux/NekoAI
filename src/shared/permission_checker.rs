use std::sync::Arc;

use serenity::all::{Cache, GuildId, Http, Permissions, UserId};

/// ボットの権限を確認するユーティリティ
pub struct PermissionChecker;

impl PermissionChecker {
    /// ボットが指定されたギルドで指定された権限を持っているか確認する
    pub async fn bot_has_permissions(
        cache: &Arc<Cache>,
        http: &Arc<Http>,
        guild_id: GuildId,
        required: Permissions,
    ) -> Result<bool, String> {
        let bot_id = cache.current_user().id;

        let member = guild_id
            .member(http.as_ref(), bot_id)
            .await
            .map_err(|e| format!("Failed to fetch bot member info: {e}"))?;

        let guild = cache
            .guild(guild_id)
            .ok_or_else(|| "Guild not found in cache".to_string())?;

        let permissions = guild.member_permissions(&member);
        Ok(permissions.contains(required))
    }

    /// 指定ユーザーがギルドで管理者権限を持っているか確認する
    pub async fn user_is_admin(
        cache: &Arc<Cache>,
        http: &Arc<Http>,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<bool, String> {
        Self::user_has_permissions(cache, http, guild_id, user_id, Permissions::ADMINISTRATOR).await
    }

    /// 指定ユーザーがギルドで指定された権限を持っているか確認する
    pub async fn user_has_permissions(
        cache: &Arc<Cache>,
        http: &Arc<Http>,
        guild_id: GuildId,
        user_id: UserId,
        required: Permissions,
    ) -> Result<bool, String> {
        let member = guild_id
            .member(http.as_ref(), user_id)
            .await
            .map_err(|e| format!("Failed to fetch member info: {e}"))?;

        let guild = cache
            .guild(guild_id)
            .ok_or_else(|| "Guild not found in cache".to_string())?;

        let permissions = guild.member_permissions(&member);

        // 管理者権限を持つユーザーは全権限を持つ
        if permissions.administrator() {
            return Ok(true);
        }

        Ok(permissions.contains(required))
    }
}
