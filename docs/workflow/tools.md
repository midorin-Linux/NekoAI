# `nekoai-tools` クレートのワークフロー

## 役割

`nekoai-tools` は、Rig SDK の `Tool` trait を実装したエージェント用ツールの集まりです。Discord API 連携ツールを `nekoai_tools::discord` モジュールで提供し、`AgentRuntime` 経由で動的に登録・実行されます。

## 主な構成

```
nekoai-rs/tools/src/
├── lib.rs              # pub mod discord;
└── discord/
    ├── mod.rs          # モジュール宣言 + pub use 再エクスポート
    ├── error.rs        # DiscordToolError（共通エラー型）
    ├── helpers.rs      # 引数パース・出力構築の共通ヘルパー
    ├── permission.rs   # 管理者権限ガード（admin_guard! マクロ群）
    ├── message.rs      # メッセージ送信・編集・削除・履歴・ピン・リアクション
    ├── channel.rs      # チャンネル作成・削除・変更・情報取得・一覧・権限
    ├── guild.rs        # ギルド情報・一覧・変更・監査ログ・BAN管理
    ├── role.rs         # ロール一覧・作成・削除・変更・メンバー付与/剥奪
    ├── member.rs       # メンバー一覧・情報・Kick/Ban・変更・Timeout
    ├── thread.rs       # スレッド作成・削除・一覧・メンバー追加
    ├── voice.rs        # ボイス移動・切断・ミュート・デフ・ステージ管理
    ├── invite.rs       # 招待一覧・作成・削除
    ├── emoji.rs        # 絵文字一覧・作成・削除、ステッカー一覧、リアクション統計
    └── schedule.rs     # イベント検索・作成・変更・キャンセル
```

## ツール実装パターン

各ツールは `rig::tool::Tool` trait を実装します。型付き引数（`Deserialize`）と `Value` ベースの2パターンがあります。

```rust
// パターンA: 型付き引数
pub struct SendDiscordMessage { http: Arc<Http> }

#[derive(Deserialize)]
pub struct SendMessageArgs {
    pub channel_id: u64,
    pub message: String,
}

impl Tool for SendDiscordMessage {
    const NAME: &'static str = "send_discord_message";
    type Error = DiscordToolError;
    type Args = SendMessageArgs;
    type Output = SendMessageOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition { /* ... */ }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> { /* ... */ }
}
```

```rust
// パターンB: Value ベース（動的パース）
pub struct EditDiscordMessage { http: Arc<Http> }

impl Tool for EditDiscordMessage {
    const NAME: &'static str = "edit_discord_message";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition { /* ... */ }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let channel_id = get_channel_id(&args, "channel_id")...;
        crate::admin_guard_channel!(&self.http, channel_id);
        // ...
    }
}
```

### 設計方針

- **引数**: 単純なツールは `serde_json::Value`（JSON スキーマでバリデーションを委譲）、`send_discord_message` などは型付き
- **出力**: `serde_json::Value`（統一フォーマット `{ ok: true, data: ... }` / `{ ok: false, error: ... }`）または専用出力型（`SendMessageOutput`）
- **エラー**: `DiscordToolError` は非致命的エラー用。API エラーは `Ok(err(...))` で返す
- **HTTP クライアント**: 各ツールが `Arc<Http>` を保持し、`Arc::clone()` で共有
- **権限チェック**: 管理者専用ツールは `admin_guard_*!` マクロでチェック

## 共通ヘルパー（`helpers.rs`）

### 出力構築

```rust
pub fn ok(data: Value) -> Value;      // { ok: true, data: ... }
pub fn err(message: impl ToString) -> Value;  // { ok: false, error: ... }
pub fn to_value<T: Serialize>(value: &T) -> Value;
```

### 引数パース（Value ベース）

```rust
pub fn get_u64(args: &Value, key: &str) -> Option<u64>;
pub fn get_u32(args: &Value, key: &str) -> Option<u32>;
pub fn get_u16(args: &Value, key: &str) -> Option<u16>;
pub fn get_u8(args: &Value, key: &str) -> Option<u8>;
pub fn get_bool(args: &Value, key: &str) -> Option<bool>;
pub fn get_string(args: &Value, key: &str) -> Option<String>;
pub fn get_u64_list(args: &Value, key: &str) -> Option<Vec<u64>>;
pub fn get_channel_id(args: &Value, key: &str) -> Option<ChannelId>;
pub fn get_user_id(args: &Value, key: &str) -> Option<UserId>;
pub fn get_message_id(args: &Value, key: &str) -> Option<MessageId>;
pub fn get_guild_id_default(args: &Value) -> Option<GuildId>;
```

### Enum 変換

```rust
pub fn parse_channel_type(value: &Value) -> Option<ChannelType>;
pub fn parse_thread_type(value: &Value) -> Option<ChannelType>;
pub fn parse_auto_archive_duration(value: &Value) -> Option<AutoArchiveDuration>;
pub fn parse_scheduled_event_type(value: &Value) -> Option<ScheduledEventType>;
pub fn parse_scheduled_event_status(value: &Value) -> Option<ScheduledEventStatus>;
pub fn parse_timestamp(value: &Value) -> Option<Timestamp>;
pub fn parse_colour(value: &Value) -> Option<Colour>;
pub fn parse_reaction_type(value: &Value) -> Option<ReactionType>;
pub fn parse_relative_time(duration_str: &str) -> Option<Duration>;
```

### リトライ

```rust
pub async fn retry_discord<F, Fut, T>(f: F) -> serenity::Result<T>;
// ExponentialBackoff(100ms~10s) + jitter, 5回リトライ
```

### ID解決

```rust
pub async fn resolve_user_id(http, guild_id, query) -> Option<UserId>;
pub async fn resolve_role_id(http, guild_id, query) -> Option<RoleId>;
pub async fn resolve_role_ids(http, guild_id, queries) -> Vec<RoleId>;
// query: "name", "@mention", "123456789" のいずれかを受け付ける
```

### その他

```rust
pub fn resolve_relative_timestamp(duration_str: &str) -> Option<Timestamp>;  // "10m", "1h", "1d" → Timestamp
pub fn snowflake_to_datetime(snowflake: u64) -> DateTime<Utc>;
pub async fn fetch_guild_members(http, guild_id, limit) -> Result<Vec<Member>>;  // ページネーション対応
```

## 権限モジュール（`permission.rs`）

ツール実行前に管理者権限をチェックするための関数とマクロを提供します。

```rust
pub async fn require_admin(http, guild_id, user_id) -> Result<(), String>;
pub async fn require_current_user_admin(http, guild_id) -> Result<(), String>;
pub async fn require_current_user_admin_for_channel(http, channel_id) -> Result<(), String>;
pub async fn require_current_user_admin_for_invite_code(http, code) -> Result<(), String>;
```

### マクロ

```rust
admin_guard_guild!($http, $guild_id);     // 管理者チェック + return Ok(err(...))
admin_guard_channel!($http, $channel_id);
admin_guard_invite!($http, $code);
```

権限不足時は `return Ok(err("..."))` で即座にエラー応答を返します。

## ツール一覧

### Low-level tools（`discord_` 接頭辞）

| モジュール | ツール名 | 説明 |
|---|---|---|
| **message** | `send_discord_message` | メッセージ送信（型付き引数） |
| | `edit_discord_message` | メッセージ編集 |
| | `delete_discord_message` | メッセージ削除 |
| | `get_discord_message` | 特定メッセージ取得 |
| | `bulk_delete_discord_messages` | 複数メッセージ一括削除 |
| | `get_discord_message_history` | メッセージ履歴取得 |
| | `pin_discord_message` | メッセージピン留め |
| | `unpin_discord_message` | メッセージピン解除 |
| | `add_discord_reaction` | リアクション追加 |
| | `remove_discord_reaction` | リアクション削除 |
| **channel** | `create_discord_channel` | チャンネル作成 |
| | `delete_discord_channel` | チャンネル削除 |
| | `modify_discord_channel` | チャンネル設定変更 |
| | `get_discord_channel_info` | チャンネル情報取得 |
| | `get_discord_channel_list` | チャンネル一覧取得 |
| **guild** | `get_discord_guild_info` | ギルド情報取得 |
| | `get_discord_guild_list` | ギルド一覧取得 |
| | `modify_discord_guild` | ギルド設定変更 |
| | `get_discord_audit_log` | 監査ログ取得 |
| **role** | `get_discord_role_list` | ロール一覧 |
| | `create_discord_role` | ロール作成 |
| | `delete_discord_role` | ロール削除 |
| | `modify_discord_role` | ロール変更 |
| | `add_discord_role_to_member` | ロール付与 |
| | `remove_discord_role_from_member` | ロール剥奪 |
| **member** | `get_discord_member_list` | メンバー一覧 |
| | `get_discord_member_info` | メンバー情報 |
| | `kick_discord_member` | メンバーKick |
| | `ban_discord_member` | メンバーBAN |
| | `unban_discord_member` | BAN解除 |
| | `bulk_ban_discord_members` | 複数BAN |
| | `modify_discord_member` | メンバー設定変更 |
| | `timeout_discord_member` | タイムアウト |
| **thread** | `create_discord_thread` | スレッド作成 |
| | `delete_discord_thread` | スレッド削除 |
| | `get_discord_thread_list` | アクティブスレッド一覧 |
| | `add_discord_thread_member` | スレッドメンバー追加 |
| **voice** | `move_discord_member_voice` | ボイスチャンネル移動 |
| | `disconnect_discord_member_voice` | ボイスから切断 |
| | `mute_discord_member` | サーバーミュート |
| | `deafen_discord_member` | サーバーデフ |
| **invite** | `get_discord_invite_list` | 招待一覧 |
| | `create_discord_invite` | 招待URL作成 |
| | `delete_discord_invite` | 招待削除 |
| **emoji** | `get_discord_emoji_list` | 絵文字一覧 |
| | `create_discord_emoji` | 絵文字作成 |
| | `delete_discord_emoji` | 絵文字削除 |
| | `get_discord_sticker_list` | ステッカー一覧 |
| **schedule** | `search_discord_scheduled_events` | 予定イベント検索（スコア付き） |
| | `schedule_discord_event` | イベント作成（自然言語日時対応） |
| | `update_discord_scheduled_event` | イベント変更（名前/キーワード解決） |
| | `cancel_discord_scheduled_event` | イベントキャンセル |

### High-level tools（エージェント向けラッパー）

内部で `admin_guard` チェックを追加し、より直感的なインターフェースを提供します。

| モジュール | ツール名 | 説明 |
|---|---|---|
| **message** | `send_message` | メッセージ送信（admin_guard付き） |
| | `search_messages` | キーワード検索 |
| | `bulk_delete_messages` | 一括削除（admin_guard付き） |
| | `pin_message` | ピン・アンピン・一覧（統合） |
| | `add_reaction` | リアクション追加（admin_guard付き） |
| | `send_webhook_message` | Webhook送信 |
| | `fetch_readable_chat_history` | LLM向け整形済み履歴 |
| | `search_channel_messages` | チャンネル内メッセージ検索 |
| | `create_poll` | 投票作成（自動リアクション付き） |
| | `send_announcement_with_pin` | お知らせ送信＋自動ピン |
| **channel** | `list_channels` | チャンネル一覧（情報付き） |
| | `create_channel` | チャンネル作成（admin_guard付き） |
| | `update_channel` | チャンネル更新（admin_guard付き） |
| | `archive_channel` | チャンネルアーカイブ（読み取り専用化） |
| | `set_channel_permissions` | チャンネル権限設定 |
| **guild** | `get_guild_info` | ギルド情報 |
| | `update_guild_settings` | ギルド設定更新（admin_guard付き） |
| | `get_audit_log` | 監査ログ取得 |
| | `manage_bans` | BAN管理（一覧/追加/削除） |
| **role** | `list_roles` | ロール一覧 |
| | `upsert_role` | ロール作成/更新（role_id有無で自動切替） |
| | `assign_roles` | 複数メンバーにロール付与/剥奪 |
| | `reorder_roles` | ロール並び替え |
| | `list_role_members` | ロール保持メンバー一覧 |
| | `assign_role_by_name` | 名前でユーザー/ロール解決して付与 |
| | `revoke_role_by_name` | 名前でユーザー/ロール解決して剥奪 |
| | `get_members_with_role` | ロール保持メンバー検索 |
| | `clear_role_from_all_members` | 全メンバーからロール一括削除 |
| | `assign_role_to_multiple_members` | 複数ユーザーに一括ロール付与 |
| | `create_and_assign_role` | ロール作成＋即時付与 |
| | `duplicate_role` | 既存ロールの複製 |
| **member** | `search_members` | 名前/ロール/Timeout状態で検索 |
| | `manage_member_roles` | 名前で解決してロール操作 |
| | `timeout_member` | 相対時間でTimeout設定（"10m", "1h", "1d"） |
| | `investigate_member` | メンバー詳細プロファイル |
| | `moderate_member` | Kick/Ban/Softban統合 |
| | `get_member_activity` | メンバーアクティビティ情報 |
| | `update_member_nickname` | ニックネーム変更 |
| | `kick_member` | Kick（admin_guard付き） |
| **thread** | `create_thread` | スレッド作成（admin_guard付き） |
| | `list_threads` | アクティブスレッド一覧 |
| | `archive_or_lock_thread` | スレッドアーカイブ/ロック |
| | `manage_thread_members` | スレッドメンバー追加/削除/一覧 |
| **voice** | `get_voice_states` | ボイスチャンネル一覧 |
| | `move_member_to_voice` | メンバーボイス移動 |
| | `set_voice_mute_deafen` | ミュート/デフ一括設定 |
| | `manage_stage_topic` | ステージトピック管理・スピーカー招待 |
| **invite** | `create_invite` | 招待作成（制限付き） |
| | `list_invites` | 招待一覧 |
| | `revoke_invite` | 招待削除 |
| **emoji** | `list_emojis` | 絵文字一覧 |
| | `add_emoji` | 絵文字追加 |
| | `delete_emoji` | 絵文字削除 |
| | `get_reaction_stats` | メッセージリアクション統計 |
| **schedule** | `list_events` | イベント一覧 |
| | `create_scheduled_event_tool` | イベント作成 |
| | `update_or_cancel_event` | イベント更新/キャンセル |
| | `get_event_subscribers` | イベント参加者一覧 |

## 起動時ワークフロー（`DiscordClient::new`）

1. `Arc<Http>` を生成
2. `register_tools!` マクロで各カテゴリのツールを `AgentRuntime::add_tool()` 経由で登録
3. ツールは `Arc::clone()` された `Http` を共有

```rust
let http = Arc::new(Http::new(&discord_token));
register_tools!(
    SendDiscordMessage::new(http.clone()),
    EditDiscordMessage::new(http.clone()),
    // ... 全ツール（low-level + high-level）
);
```

## ツール新規追加の手順

1. 対応するモジュールファイル（例: `channel.rs`）に新しいツール構造体を追加
2. `Tool` trait を実装（`definition` + `call`）
3. 管理者権限が必要なら `admin_guard_*!` マクロを `call()` 内で呼ぶ
4. `mod.rs` の `pub use` と `pub mod` に追加
5. `client.rs` の `register_tools!` マクロに追加
6. （必要に応じて）`helpers.rs` にパーサー関数を追加

## 連携ポイント

- `nekoai-agent`: `ToolServerHandle` を介したツール実行
- `nekoai-discord`: 起動時に全ツールを `AgentRuntime::add_tool()` で登録
- `serenity`: Discord API 呼び出し基盤
