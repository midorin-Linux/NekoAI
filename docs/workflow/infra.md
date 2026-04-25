# `nekoai-infra` クレートのワークフロー

## 役割

`nekoai-infra` は横断的な基盤機能を提供します。現時点ではロギング初期化に特化しており、アプリ起動時に一度だけ呼ばれる想定です。

## 主な構成

- `logging.rs`: `tracing` 初期化とログファイル設定

## ログ初期化ワークフロー（`init_tracing`）

1. `logs` ディレクトリの存在を確認
2. 存在しなければ `logs` を作成
3. 現在時刻を含むログファイル名 `logs/nekoai-YYYY-MM-DD_HH-MM-SS.log` を生成
4. `tracing_appender::non_blocking` で非同期書き込みを構成
5. `.env` を読み込み（`dotenvy::dotenv()`）
6. `LOG_LEVEL` 環境変数から `EnvFilter` を構築（未設定時は `info`）
7. `tracing_subscriber::fmt()` を設定
   - writer: 上記 non-blocking
   - env filter: `LOG_LEVEL`
   - ANSI: 無効（ファイルログ向け）
8. `WorkerGuard` を返却

`WorkerGuard` は drop 時にバッファフラッシュされるため、呼び出し側で保持が必要です。

## 利用ワークフロー

1. `nekoai-cli` の `StartCommand::start` が `init_tracing` を呼ぶ
2. 返却された `WorkerGuard` を `StartCommand` に保持
3. 以降の全クレートの `tracing` ログが同一ログファイルに出力される

## エラー時の挙動

- ディレクトリ作成失敗: `Result::Err`
- ログファイル作成失敗: `Result::Err`
- 呼び出し側は起動を停止

## 今後の拡張余地

- stdout とファイルの二重出力
- JSON ログフォーマット
- OpenTelemetry 連携
