# `nekoai-config` クレートのワークフロー

## 役割

`nekoai-config` は設定スキーマ定義と設定ファイルロードを担当します。アプリ全体で利用される `Config` 構造体を提供し、各クレートはこの型を介して設定値を参照します。

## 主な構成

- `loader.rs`: すべての設定型とロード処理

## 設定スキーマワークフロー

`loader.rs` で以下の設定ツリーを定義しています。

- `chat_platform`
  - `discord`（デフォルト）
- `discord`
  - `token`, `guild_id`
- `provider.language_model`
  - `provider_base_url`, `api_key`, `model_name`, `parameters`
- `provider.embedding_model`
  - `provider_base_url`, `api_key`, `model_name`, `dimension`
- `memory`
  - `short_term_max_entries`, `mid_term_top_k`, `long_term_top_k`, `mid_term_retention_days`
- `memory.vector_db`
  - `url`, `api_key`, `mid_term_collection`, `long_term_collection`

## ロードワークフロー（`Config::load`）

1. `config::ConfigBuilder` を生成
2. `.config/config.json` を必須ソースとして追加
3. JSON として読み込み
4. `serde` で `Config` にデシリアライズ
5. 成功時は `Config` を返却

読み込み元は現状 1 ファイル固定で、環境変数オーバーライドは未実装です。

## デフォルト値ワークフロー

以下のキーは `#[serde(default)]` または既定関数で補完されます。

- `chat_platform`: `discord`
- `memory.vector_db.url`: `http://localhost:6334`
- `memory.vector_db.mid_term_collection`: `mid_term`
- `memory.vector_db.long_term_collection`: `long_term`
- `memory.short_term_max_entries`: `20`
- `memory.mid_term_top_k`: `3`
- `memory.long_term_top_k`: `5`
- `memory.mid_term_retention_days`: `30`

## エラー時の挙動

- ファイル不在: `ConfigError` を返却
- JSON 形式不正: `ConfigError` を返却
- 型不一致: `ConfigError` を返却

呼び出し側（CLI）がこれを受けて起動を停止します。

## 連携ポイント

- `nekoai-cli`: 起動時ロード
- `nekoai-agent`: 推論モデル/API/パラメータ参照
- `nekoai-memory`: 埋め込みモデル・Qdrant 設定参照
