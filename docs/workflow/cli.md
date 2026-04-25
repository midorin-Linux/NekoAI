# `nekoai-cli` クレートのワークフロー

## 役割

`nekoai-cli` はアプリケーションのエントリポイントです。CLI コマンド解釈、起動前初期化、進捗表示、チャットプラットフォーム起動までを担当します。

## 主な構成

- `main.rs`: コマンド定義と実行分岐
- `commands/start.rs`: 起動手順の実体
- `chat.rs`: チャットプラットフォーム（現在は Discord）の抽象

## コマンドワークフロー

`neko` コマンドは現在 `start` サブコマンドのみを持ちます。

1. `clap` で引数を解析
2. `start` が選択されたら `StartCommand::new().await`
3. 初期化成功後、`AgentRuntime::new_with_progress(...)` を実行
4. `ChatClient::initialize(...)` でプラットフォーム別クライアント生成
5. `chat_client.run().await` でイベントループ開始

失敗時はエラー表示して `exit(1)`、正常終了時は `exit(0)` です。

## `start` の詳細ワークフロー

`StartCommand::start` は以下の順で処理します。

1. ASCII バナーを表示
2. `init_tracing()` を実行し、ログ出力を初期化
3. `.config/config.json` の存在確認
4. 設定がない場合は対話でセットアップ実行意志を確認
   - `y`: 継続（現時点ではウィザード起動実装は TODO）
   - `n`: 終了
5. `Config::load()` で設定をロード
6. `MemoryStore::new(&config)` を生成
7. `memory_store.initialize().await` でベクトルコレクションを準備
8. `memory_store.start_cleanup_job()` で中期記憶の定期クリーンアップを開始
9. `(config, tracing_guard, memory_store)` を返却

処理中は `indicatif` のスピナーで状態を表示します。

## `AgentRuntime` 初期化連携

`main.rs` 側では `RuntimeInitProgress` を使って進捗バーを更新します。

- 総ステップ数: `RuntimeInitProgress::TOTAL_STEPS`（5）
- 各ステップのメッセージをそのままバー表示
- 成功後に「Agent runtime initialized」を表示

## チャットクライアント選択ワークフロー

`ChatClient::initialize` は `config.chat_platform` で分岐します。

- `ChatPlatform::Discord` の場合:
  1. `DiscordClient::new(token, guild_id, runtime)`
  2. `ChatClient::Discord(client)` を返却

`ChatClient::run` は enum を展開し、該当クライアントの `run` を呼び出します。

## エラー時の挙動

- 起動前初期化の失敗は即時終了
- ログ初期化失敗時は明示メッセージを出して終了
- 設定読み込み失敗時はユーザー向けに原因を表示

## 連携ポイント

- `nekoai-config`: 設定読み込み
- `nekoai-infra`: ロギング初期化
- `nekoai-memory`: 記憶層初期化
- `nekoai-agent`: 推論ランタイム
- `nekoai-discord`: Discord クライアント
