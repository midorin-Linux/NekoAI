# nekoai-config レビュー

## 対象
- `crates/config/src/loader.rs`

## 指摘事項

### 1. [Medium] 設定値のバリデーションがなく、不正値がそのまま通る
- 該当: `crates/config/src/loader.rs:21`, `crates/config/src/loader.rs:136`
- `temperature` / `top_p` / `max_token` / `dimension` などに範囲検証がないため、実行時にプロバイダエラーへ遅延します。
- 修正案:
  - `Config::load()` 後に `validate()` を呼び、範囲外値を早期に弾く
  - 例: `0.0 <= temperature <= 2.0`, `0.0 < top_p <= 1.0`, `max_token > 0`, `dimension > 0`

### 2. [Medium] `discord` が必須で、将来のプラットフォーム拡張時に読み込み不能
- 該当: `crates/config/src/loader.rs:102`
- `chat_platform` に依存せず `discord` が必須なので、Discord 以外を追加した際に互換性を崩します。
- 修正案:
  - プラットフォーム別設定を `Option` 化し、`chat_platform` に応じて必須性を検証する

### 3. [Low] API キーやトークンが `Debug` 可能な型に保持されている (完了)
- 該当: `crates/config/src/loader.rs:6`, `crates/config/src/loader.rs:27`, `crates/config/src/loader.rs:35`
- 現状ログ出力はしていないものの、将来的な `{:?}` ログで機密値漏えいの余地があります。
- 修正案:
  - 機密値をラップ型にして `Debug` をマスク
  - または秘密値を含む構造体の `Debug` derive を外す
- 確認: `SecretKey` にカスタム `Debug` 実装が追加され、末尾4桁以外は `*` でマスクされるようになりました。

### 4. [Low] 設定ファイルのパスが `.config/config.json` にハードコードされており、カスタムパス指定ができない
- 該当: `crates/config/src/loader.rs:143`
- CLI引数や環境変数で設定ファイルのパスを上書きする手段がなく、テストや複数環境での設定切替が困難です。
- 修正案:
  - `Config::load()` に `path: &str` 引数を追加する
  - デフォルト値を `.config/config.json` とし、CLI から `--config` で指定可能にする
