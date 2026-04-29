# nekoai-cli レビュー

## 対象
- `crates/cli/src/main.rs`
- `crates/cli/src/chat.rs`
- `crates/cli/src/commands/start.rs`

## 指摘事項

### 1. [High] セットアップウィザード分岐が未実装のまま処理継続する
- 該当: `crates/cli/src/commands/start.rs:84`, `crates/cli/src/commands/start.rs:103`, `crates/cli/src/commands/start.rs:107`
- `y` を選んでも実際のセットアップ処理は呼ばれず、そのまま `Config::load()` に進むため初回起動時に失敗します。
- 修正案:
  - 設定未存在時は実際のウィザード実行関数を呼び出す
  - 未実装の場合は明示的に「未実装」と表示して終了し、誤動作を防ぐ

### 2. [Medium] `std::process::exit` 多用で終了処理がスキップされる (完了)
- 該当: `crates/cli/src/main.rs:30`, `crates/cli/src/main.rs:97`, `crates/cli/src/commands/start.rs:56`
- `process::exit` は `Drop` を実行しないため、`WorkerGuard` などの後始末が走らずログ欠落の原因になります。
- 修正案:
  - `main` を `Result<()>` で返し、終了コード制御は最外周に限定する
  - 失敗は `?` で伝播し、最後に 1 箇所で終了処理する

### 3. [Medium] 設定ファイル存在チェックで I/O エラーを握りつぶしている (完了)
- 該当: `crates/cli/src/commands/start.rs:75`
- `if let Ok(false)` だと `Err`（権限エラー等）を見落とし、後段で分かりにくい失敗になります。
- 修正案:
  - `match std::fs::exists(...)` で `Err` を明示ハンドリングする
  - パスと原因を含むエラーメッセージを返す

### 4. [Low] 未使用の短期記憶インスタンス生成が残っている (完了)
- 該当: `crates/cli/src/commands/start.rs:132`
- `MemoryStore` 側でも短期記憶を管理しているため、ここでの `ShortTermMemory::new(10)` は実際に使われません。
- 修正案:
  - 不要な生成コードを削除し、責務を `MemoryStore` に一本化する

### 5. [Medium] セットアップウィザードで `y` 選択後も実際のウィザード実装がなく設定未作成のまま `Config::load()` に進む（#1 の詳細・未完了）
- 該当: `crates/cli/src/commands/start.rs:93`
- ユーザーが `y` を選択しても「Starting setup wizard...」と表示するだけで、実際の設定ファイル作成処理が実行されないため、後続の `Config::load()` が失敗します。
- 修正案:
  - 実際のウィザード処理（対話式入力による `config.json` の生成）を実装する
  - 未実装のままなら明示的にエラーとして終了し、誤動作を防ぐ

### 6. [Low] グレースフルシャットダウンが未実装（Ctrl+C / SIGTERM ハンドリングなし）
- 該当: `crates/cli/src/main.rs:22`
- `AgentRuntime::shutdown()` が呼ばれず、プロセス強制終了時に進行中の長期記憶抽出タスクが中断される可能性があります。
- 修正案:
  - `tokio::signal::ctrl_c()` でシグナルを捕捉し、`runtime.shutdown().await` を呼び出してから終了する
