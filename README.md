# NekoAI

NekoAI は Rust で開発されている **Discord 用 AI エージェント** です。
[Rig SDK](https://github.com/0xPlaygrounds/rig) を基盤とし、強力なメモリ管理（長期・中期・短期記憶）とツール実行機能を備えた、拡張性の高いチャットボットを提供します。

## 主な特徴

- **階層型メモリシステム:**
    - **短期記憶:** 最新の会話コンテキストを DashMap でインメモリ保持。
    - **中期記憶:** 過去の会話の要約を Qdrant 等のベクトル DB で管理。
    - **長期記憶:** ユーザーに関する重要な事実を永続化。
- **マルチモーダル対話:** スラッシュコマンド（`/ask`, `/clear`）による直感的な操作。
- **堅牢なアーキテクチャ:** Cargo Workspace を採用し、ドメイン・インフラ・アプリケーション層を明確に分離。
- **インタラクティブな起動:** 初回起動時に CLI によるセットアップウィザードを提供。

## 技術スタック

- **[Rig](https://github.com/0xPlaygrounds/rig)**: エージェント構築・LLM 抽象化
- **[Serenity](https://github.com/serenity-rs/serenity)**: Discord API クライアント
- **[Qdrant](https://qdrant.tech/)**: ベクトル検索エンジン（記憶層）
- **[Tokio](https://tokio.rs/)**: 非同期ランタイム
- **[Tracing](https://github.com/tokio-rs/tracing)**: 構造化ログ

## ディレクトリ構造

プロジェクトは複数のクレートで構成されています。詳細は [architecture.md](docs/architecture.md) を参照してください。

```text
NekoAI/
├── crates/
│   ├── agent/       # Rig を使用したエージェントの推論ループ・セッション管理
│   ├── cli/         # エントリポイント・コマンドラインインターフェース
│   ├── config/      # 設定ファイルのロード・検証 (.config/config.json)
│   ├── discord/     # Serenity による Discord イベント処理・コマンドルーティング
│   ├── domain/      # ビジネスロジック・共通のデータ型定義
│   ├── infra/       # ログ、DB 接続、外部サービスとの通信
│   ├── memory/      # 3層記憶（短期・中期・長期）の統合インターフェース
│   ├── tools/       # エージェントが実行可能なツール群
│   └── (setup)/     # 初回 TUI セットアップウィザード（予定）
├── docs/            # アーキテクチャ設計書・ドキュメント
└── target/          # ビルド成果物
```

## セットアップ

### 1. 必要条件
- [Rust](https://www.rust-lang.org/) (latest stable)
- [Qdrant](https://qdrant.tech/) (ベクトル検索を使用する場合)
- [just](https://github.com/casey/just)

### 2. インストール
```bash
git clone https://github.com/midorin-Linux/NekoAI.git
cd NekoAI
```

### 3. 設定
初回起動時に設定ファイル `.config/config.json` がない場合、インタラクティブなセットアップが開始されます。

### 4. 実行
```bash
# 開発用
just neko start
```

## 開発

### フォーマットとリンター
```bash
just fmt
cargo clippy -- -D warnings
```

## ライセンス
[Apache-2.0](LICENSE)
