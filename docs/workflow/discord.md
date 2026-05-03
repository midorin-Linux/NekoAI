# `nekoai-discord` クレートのワークフロー

## 役割

`nekoai-discord` は Discord 接続、イベントループ、コマンド受付、`AgentRuntime` への委譲を担当します。推論そのものは持たず、入出力とルーティングに集中します。

## 主な構成

- `client.rs`: Serenity クライアント生成、起動、ツール登録
- `handler.rs`: `EventHandler` 実装（ready イベント）
- `command_router.rs`: Poise フレームワーク設定
- `commands/*`: `/ask`, `/clear`, `/history` の実装
- `commands/utils/session_resolver.rs`: Discord コンテキストから `SessionKey` 判定

## クライアント起動ワークフロー（`DiscordClient::new`）

1. 受け取った `discord_token`, `guild_id`, `agent_runtime` を使用
2. 必要な Gateway Intents を設定（`GUILDS`, `GUILD_MESSAGES`, `MESSAGE_CONTENT`）
3. コマンドフレームワーク（`command_framework`）を構築
4. `Handler` をイベントハンドラとして登録
5. Serenity `Client` を生成して返却
6. `Arc<Http>` を生成
7. `register_tools!` マクロで全 Discord ツールを `AgentRuntime::add_tool()` 経由で登録

### ツール登録詳細

`client.rs` では `register_tools!` マクロでカテゴリごとにツールを登録します：

- **メッセージ系**: `send`, `edit`, `delete`, `get`, `bulk_delete`, `history`, `pin`, `unpin`, `add_reaction`, `remove_reaction`
- **チャンネル系**: `create`, `delete`, `modify`, `get_info`, `get_list`
- **ギルド系**: `get_info`, `get_list`, `modify`, `get_audit_log`
- **ロール系**: `get_list`, `create`, `delete`, `modify`, `add_to_member`, `remove_from_member`
- **メンバー系**: `get_list`, `get_info`, `kick`, `ban`, `unban`, `bulk_ban`, `modify`, `timeout`
- **スレッド系**: `create`, `delete`, `get_list`, `add_member`
- **ボイス系**: `move`, `disconnect`, `mute`, `deafen`
- **招待系**: `get_list`, `create`, `delete`
- **絵文字系**: `get_list`, `create`, `delete`, `get_sticker_list`
- **イベント系**: `get_list`, `create`, `modify`, `delete`

各ツールは `Arc<Http>` のクローンを共有し、`AgentRuntime::add_tool()` で Rig の `ToolServerHandle` に登録されます。

`run()` では `discord_client.start().await` を呼び出し、イベントループに入ります。

## フレームワーク構築ワークフロー（`command_framework`）

1. コマンド一覧 `ask`, `clear`, `history` を登録
2. Prefix コマンド接頭辞を `w!` に設定
3. `on_error`, `pre_command`, `post_command` を設定
4. `setup` 内で対象 guild へコマンドを登録
5. `Data { agent_runtime }` をコンテキストに注入

## `/ask` ワークフロー

1. Bot ユーザーの実行を除外
2. Slash の場合 `defer()`、Prefix の場合 typing 開始
3. `session_resolver` で `SessionKind` と `thread_id` を判定
4. `SessionKey` を生成
5. `agent_runtime.submit(session_key, user_id, prompt)` を呼び出し
6. 返信テキストを整形（ユーザー発話 + Assistant 応答）
7. 2000 文字上限で `split_message` し、複数メッセージ送信

## `/clear` ワークフロー

1. Bot 実行を除外
2. `defer()`
3. `SessionKey` を解決
4. `agent_runtime.clear_session(&session_key).await`
5. 成功/失敗メッセージを返す

## `/history` ワークフロー

1. Bot 実行を除外
2. `defer()`
3. `SessionKey` を解決
4. `agent_runtime.get_history(&session_key)`
5. ターン履歴を整形して送信

## セッション解決ワークフロー（`session_resolver`）

`ChannelId` から Discord チャンネル種別を取得し、次を判定します。

- Public/Private/News Thread: `SessionKind::Thread`, `thread_id=Some(channel_id)`
- Guild 通常チャンネル: `SessionKind::GuildChannel`
- DM: `SessionKind::DirectMessage`
- 判定失敗時: `guild_id` の有無で Guild/DM をフォールバック

## エラー時の挙動

- コマンド内部エラーは `on_error` または各コマンド内でログ出力
- `/ask` 実行失敗時はエラー文字列を返信
- `/clear`, `/history` は失敗時に固定エラーメッセージを返信

## 連携ポイント

- `nekoai-agent`: 推論・履歴・セッションクリア、ツール実行
- `nekoai-tools`: Discord API 連携ツール群（`nekoai_tools::discord::*`）
- `nekoai-domain`: `SessionKey` / `SessionKind`
- `serenity` / `poise`: Discord API とコマンド実行基盤
