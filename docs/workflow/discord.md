# `nekoai-discord` クレートのワークフロー

## 役割

`nekoai-discord` は Discord 接続、イベントループ、コマンド受付、`AgentRuntime` への委譲を担当します。推論そのものは持たず、入出力とルーティングに集中します。

## 主な構成

- `client.rs`: Serenity クライアント生成、起動、全 Discord ツールの登録（`register_tools!` マクロ内の一括登録）
- `handler.rs`: `EventHandler` 実装（ready イベント → スピナー停止）
- `command_router.rs`: Poise フレームワーク設定（`on_error`, `pre_command`, `post_command` フック付き）
- `commands/ask.rs`: `/ask` コマンド（prefix + slash）
- `commands/clear.rs`: `/clear` コマンド（prefix + slash）
- `commands/history.rs`: `/history` コマンド（slash のみ）
- `commands/utils/session_resolver.rs`: Discord コンテキストから `SessionKey` 判定

## クライアント起動ワークフロー（`DiscordClient::new`）

1. 受け取った `discord_token`, `guild_id`, `agent_runtime` を使用
2. 必要な Gateway Intents を設定（`GUILDS`, `GUILD_MESSAGES`, `MESSAGE_CONTENT`）
3. コマンドフレームワーク（`command_framework`）を構築
4. `Handler` をイベントハンドラとして登録（`agent_runtime` とスピナーを保持）
5. Serenity `Client` を生成して返却
6. `Arc::new(Http::new(&discord_token))` で HTTP クライアントを生成
7. `register_tools!` ローカルマクロで全 Discord ツールを `AgentRuntime::add_tool()` 経由で動的登録

### ツール登録詳細

`client.rs` では `register_tools!` マクロ（クレートローカルな `macro_rules!`）で以下のツールを一括登録します：

- **チャンネル系**: `ListChannels`, `CreateChannelTool`, `UpdateChannel`, `ArchiveChannel`, `SetChannelPermissions`
- **絵文字系**: `ListEmojis`, `AddEmoji`, `DeleteEmoji`, `GetReactionStats`
- **ギルド系**: `GetGuildInfo`, `UpdateGuildSettings`, `GetAuditLog`, `ManageBans`
- **招待系**: `CreateInviteTool`, `ListInvites`, `RevokeInvite`
- **メンバー系**: `SearchMembers`, `UpdateMemberNickname`, `TimeoutMember`, `KickMember`, `GetMemberActivity`
- **メッセージ系**: `SendMessageTool`, `SearchMessages`, `BulkDeleteMessages`, `PinMessage`, `AddReaction`, `SendWebhookMessage`
- **ロール系**: `ListRoles`, `UpsertRole`, `AssignRoles`, `ReorderRoles`, `ListRoleMembers`
- **イベント系**: `CreateScheduledEventTool`, `ListEvents`, `UpdateOrCancelEvent`, `GetEventSubscribers`
- **スレッド系**: `CreateThreadTool`, `ListThreads`, `ArchiveOrLockThread`, `ManageThreadMembers`
- **ボイス系**: `GetVoiceStates`, `MoveMemberToVoice`, `SetVoiceMuteDeafen`, `ManageStageTopic`

各ツールは `Arc<Http>` のクローンを共有し、`AgentRuntime::add_tool()` で登録されます。

`run()` では `discord_client.start().await` を呼び出し、イベントループに入ります。

## フレームワーク構築ワークフロー（`command_framework`）

1. コマンド一覧 `ask()`, `clear()`, `history()` を登録
2. Prefix コマンド接頭辞を `w!` に設定
3. `on_error` でエラー種別ごとにログ出力（`Setup`, `Command`, `CommandCheckFailed` を個別ハンドリング、未対応のものは `poise::builtins::on_error` へ委譲）
4. `pre_command` でコマンド実行前に tracing ログ（user_id, channel_id, command名）
5. `post_command` でコマンド実行後に tracing ログ（同上）
6. `setup` 内で対象 guild へコマンドを登録
7. `Data { agent_runtime }` をコンテキストに注入

## `/ask` ワークフロー

1. Bot ユーザーの実行を除外
2. Slash の場合 `ctx.defer()`、Prefix の場合 `channel_id.start_typing()` でタイピングインジケータ
3. `session_resolver` で `SessionKind` と `thread_id` を判定
4. `SessionKey { guild_id, channel_id, thread_id, kind }` を生成
5. `agent_runtime.submit(session_key, Some(user_id), prompt)` を呼び出し
6. 返信テキストを整形（`**ユーザー名**: prompt\n\n**Assistant**: response`）
7. 2000 文字上限で `split_message`（改行で分割、なければ強制分割）し、複数メッセージ送信

## `/clear` ワークフロー

1. Bot 実行を除外
2. `ctx.defer()`
3. `SessionKey` を解決
4. `agent_runtime.clear_session(&session_key).await`
5. 成功時 "The session cleared."、失敗時 "Failed to clear the session." を返す

## `/history` ワークフロー

1. Bot 実行を除外
2. `ctx.defer()`
3. `SessionKey` を解決
4. `agent_runtime.get_history(&session_key)`
5. ターン履歴を `**User**: ...\n**Assistant**: ...` 形式で連結して送信
6. **slash_command のみ**（prefix 非対応）

## セッション解決ワークフロー（`session_resolver`）

`ChannelId` から Discord チャンネル種別を取得し、次を判定します。

- Thread（Public/Private/News）: `SessionKind::Thread`, `thread_id = Some(channel_id)`
- Guild 通常チャンネル: `SessionKind::GuildChannel`, `thread_id = None`
- DM: `SessionKind::DirectMessage`, `thread_id = None`
- 判定失敗または未知のチャンネル種別: `guild_id` の有無で `GuildChannel` / `DirectMessage` をフォールバック

## 起動時表示フロー（Handler）

1. `ready` イベント受信
2. スピナー（`indicatif::ProgressBar`）を `finish_and_clear()`
3. `"✓ Discord client ready! Logged in as {bot_name}"` を緑色で表示
4. 以降はイベントループでコマンド待機

## エラー時の挙動

- `on_error`: Setup エラーは panic、Command エラーは error ログ、CommandCheckFailed は warn ログ、その他は `poise::builtins::on_error` に委譲
- `/ask` 実行失敗時はエラー文字列をそのまま返信（`err.to_string()`）
- `/clear`, `/history` は失敗時に固定エラーメッセージを返信

## 連携ポイント

- `nekoai-agent` (`AgentRuntime`): 推論・履歴・セッションクリア、ツール実行
- `nekoai-tools` (`nekoai_tools::discord::*`): Discord API 連携ツール群（message, channel, guild, role, member, thread, voice, invite, emoji, schedule）
- `nekoai-domain` (`SessionKey` / `SessionKind`): セッション識別子の型定義
- `serenity` / `poise`: Discord API とコマンド実行基盤
- `indicatif` / `colored`: 起動時のスピナー表示と色付きログ出力
