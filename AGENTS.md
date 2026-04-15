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
