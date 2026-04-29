# nekoai-discord レビュー

## 対象
- `crates/discord/src/client.rs`
- `crates/discord/src/command_router.rs`
- `crates/discord/src/commands/*.rs`
- `crates/discord/src/commands/utils/session_resolver.rs`

## 指摘事項

### 1. [High] `history` コマンドが 2000 文字制限を超えると送信失敗する
- 該当: `crates/discord/src/commands/history.rs:42`, `crates/discord/src/commands/history.rs:49`
- `ask` は分割送信実装がある一方で `history` は一括送信のため、履歴が長いと Discord API エラーになります。
- 修正案:
  - `ask::split_message` 相当の分割処理を共通化して `history` でも利用する

### 2. [Medium] `FrameworkError::Setup` で `panic!` している (完了)
- 該当: `crates/discord/src/command_router.rs:13`
- 起動失敗時にプロセスが即時クラッシュし、CLI 側での一貫したエラーハンドリングに乗りません。
- 修正案:
  - `panic!` をやめ、`tracing::error!` + ユーザー向けメッセージに寄せる

### 3. [Medium] 必要以上に強い Gateway Intent を要求している (完了)
- 該当: `crates/discord/src/client.rs:30`
- `GUILD_MEMBERS` / `GUILD_PRESENCES` は現在実装で使用しておらず、Bot 設定側で不要な Privileged Intent を要求します。
- 修正案:
  - 実際に使う Intent のみに削減する
  - 必要になった時点で増やす

### 4. [Low] 起動時に未使用オブジェクトを作成している (完了)
- 該当: `crates/discord/src/client.rs:38`, `crates/discord/src/client.rs:41`
- `_http` / `_shared_cache` を生成していますが参照されず、意図が伝わりにくい状態です。
- 修正案:
  - 不要なら削除する
  - 将来利用予定なら理由をコメントで明示する

### 5. [Medium] `FrameworkError::Setup` の `panic!` が残存しており修正が不十分（#2 の再発）
- 該当: `crates/discord/src/command_router.rs:15`
- issue 2 は「完了」とされていますが、`tracing::error!` が追加されたのみで `panic!` は削除されていません。起動失敗時にプロセスが即時クラッシュします。
- 修正案:
  - `panic!` を削除し、エラーを上位に伝播させる
  - もしくは `Err` を返す方法に変更する

### 6. [Low] `session_resolver` が毎コマンドで Discord API の `channel_id.to_channel()` を呼び出しレイテンシを増大させている
- 該当: `crates/discord/src/commands/utils/session_resolver.rs:11`
- 全スラッシュコマンドで HTTP API コールが発生し、余分なレイテンシが追加されます。
- 修正案:
  - キャッシュ機構を導入する
  - serenity の `ChannelId` が持つチャンネル種別情報を直接利用できないか検討する

### 7. [Low] `Command` エラーハンドリングでユーザーへのエラーフィードバックがない
- 該当: `crates/discord/src/command_router.rs:17`
- コマンド実行エラーが `tracing::error!` でログ出力されるだけで、Discord 上でユーザーにエラーが伝わりません。
- 修正案:
  - `ctx.say("An error occurred while processing the command.")` などでユーザーに通知する
