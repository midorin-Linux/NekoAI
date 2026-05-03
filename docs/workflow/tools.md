# `nekoai-tools` クレートのワークフロー

## 役割

`nekoai-tools` は、Rig SDK の `Tool` trait を実装したエージェント用ツールの集まりです。Discord API 連携ツールを `nekoai_tools::discord` モジュールで提供し、`AgentRuntime` 経由で動的に登録・実行されます。

## 主な構成

```
crates/tools/src/
├── lib.rs              # pub mod discord;
└── discord/
    ├── mod.rs          # モジュール宣言 + pub use 再エクスポート
    ├── error.rs        # DiscordToolError（共通エラー型）
    ├── helpers.rs      # 引数パース・出力構築の共通ヘルパー
    ├── message.rs      # メッセージ送信・編集・削除・履歴・ピン・リアクション
    ├── channel.rs      # チャンネル作成・削除・変更・情報取得・一覧
    ├── guild.rs        # ギルド情報・一覧・変更・監査ログ
    ├── role.rs         # ロール一覧・作成・削除・変更・メンバー付与/剥奪
    ├── member.rs       # メンバー一覧・情報・Kick/Ban・変更・Timeout
    ├── thread.rs       # スレッド作成・削除・一覧・メンバー追加
    ├── voice.rs        # ボイス移動・切断・ミュート・デフ
    ├── invite.rs       # 招待一覧・作成・削除
    ├── emoji.rs        # 絵文字一覧・作成・削除、ステッカー一覧
    └── schedule.rs     # イベント一覧・作成・変更・削除
```

## ツール実装パターン

各ツールは `rig::tool::Tool` trait を実装します。

```rust
pub struct SomeTool {
    http: Arc<Http>,
}

impl SomeTool {
    pub fn new(http: Arc<Http>) -> Self {
        Self { http }
    }
}

impl Tool for SomeTool {
    const NAME: &'static str = "tool_name";
    type Error = DiscordToolError;
    type Args = Value;
    type Output = Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "...".to_string(),
            parameters: serde_json::json!({ ... }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 引数パース → serenity API 呼び出し → 結果返却
        // API エラーは Ok(err("...")) で返す（非致命的）
        // 必須引数不足も同様に Ok(err("...")) で返す
    }
}
```

### 設計方針

- **引数**: `serde_json::Value`（JSON スキーマでバリデーションを委譲）
- **出力**: `serde_json::Value`（統一フォーマット `{ ok: bool, data: ... }` / `{ ok: false, error: ... }`）
- **エラー**: `DiscordToolError` はシリアライズ失敗などの非致命的エラー用。API エラーは `Ok(err(...))` で返す
- **HTTP クライアント**: 各ツール構造体が `Arc<Http>` を保持し、`Arc::clone()` で共有

## 共通ヘルパー（`helpers.rs`）

### 出力構築

```rust
pub fn ok(data: Value) -> Value;   // { ok: true, data: ... }
pub fn err(message: impl ToString) -> Value;  // { ok: false, error: ... }
pub fn to_value<T: Serialize>(value: &T) -> Value;
```

### 引数パース

```rust
pub fn get_u64(args: &Value, key: &str) -> Option<u64>;
pub fn get_string(args: &Value, key: &str) -> Option<String>;
pub fn get_bool(args: &Value, key: &str) -> Option<bool>;
// ... 他 get_channel_id, get_user_id, get_guild_id_default など
```

### Enum 変換

```rust
pub fn parse_channel_type(value: &Value) -> Option<ChannelType>;
pub fn parse_thread_type(value: &Value) -> Option<ChannelType>;
pub fn parse_scheduled_event_type(value: &Value) -> Option<ScheduledEventType>;
pub fn parse_timestamp(value: &Value) -> Option<Timestamp>;
pub fn parse_colour(value: &Value) -> Option<Colour>;
pub fn parse_reaction_type(value: &Value) -> Option<ReactionType>;
```

## ツール一覧

| モジュール | ツール名 | 説明 |
|---|---|---|
| **message** | `send_discord_message` | メッセージ送信 |
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
| | `kick_discord_member` | メンバー蹴る |
| | `ban_discord_member` | メンバーをBAN |
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
| **schedule** | `get_discord_scheduled_events` | 予定イベント一覧 |
| | `create_discord_scheduled_event` | イベント作成 |
| | `modify_discord_scheduled_event` | イベント変更 |
| | `delete_discord_scheduled_event` | イベント削除 |

## 起動時ワークフロー（`DiscordClient::new`）

1. `Arc<Http>` を生成
2. `register_tools!` マクロで各カテゴリのツールを `AgentRuntime::add_tool()` 経由で登録
3. ツールは `Arc::clone()` された `Http` を共有

```rust
let http = Arc::new(Http::new(&discord_token));
register_tools!(
    SendDiscordMessage::new(http.clone()),
    EditDiscordMessage::new(http.clone()),
    // ... 全ツール
);
```

## ツール新規追加の手順

1. 対応するモジュールファイル（例: `channel.rs`）に新しいツール構造体を追加
2. `Tool` trait を実装（`definition` + `call`）
3. `client.rs` の `register_tools!` マクロに追加
4. （必要に応じて）`helpers.rs` にパーサー関数を追加

## 連携ポイント

- `nekoai-agent`: `ToolServerHandle` を介したツール実行
- `nekoai-discord`: 起動時に全ツールを `AgentRuntime::add_tool()` で登録
- `serenity`: Discord API 呼び出し基盤
