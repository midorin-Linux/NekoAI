# nekoai-infra レビュー

## 対象
- `crates/infra/src/logging.rs`

## 指摘事項

### 1. [High] `tracing_subscriber::fmt().init()` が多重初期化で panic する
- 該当: `crates/infra/src/logging.rs:23`
- テストや再初期化経路で `init_tracing()` が複数回呼ばれると panic し、復旧不能になります。
- 修正案:
  - `.try_init()` を使って `Result` で返す
  - 既初期化時は warning を出して継続する
- 補足: 以前「不要」とされていましたが、結合テストや複数回の初期化経路を考慮すると実際に問題になる可能性があります。

### 2. [Medium] ログディレクトリ作成で I/O エラーを握りつぶしている (完了)
- 該当: `crates/infra/src/logging.rs:10`
- `if let Ok(false)` で `exists` の `Err` を無視するため、後続のファイル作成時に原因不明エラーになりやすいです。
- 修正案:
  - `match std::fs::exists("logs")` で `Err` を即時返却する

### 3. [Low] ログローテーション不在で単一ファイルが肥大化する (完了)
- 該当: `crates/infra/src/logging.rs:15`
- 現状は起動単位のファイル作成のみで、長時間運用のサイズ制御がありません。
- 修正案:
  - `tracing-appender` の rolling appender（日次/時間単位）を利用する
- 確認: `rolling::daily("logs", "nekoai.log")` により日次ローテーションが実装されています。

### 4. [Low] `dotenvy::dotenv().ok()` が `.env` ファイルの読み込みエラーを握りつぶしている
- 該当: `crates/infra/src/logging.rs:17`
- `.env` ファイルが存在しない場合は何もしない正常動作ですが、ファイルが破損している場合などのエラーが静かに無視されます。
- 修正案:
  - エラーの内容を `debug!` または `warn!` でログ出力する
