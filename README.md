# NekoAI

NekoAI は Rust で開発されている **Discord 用 AI エージェント** です。
[Rig SDK](https://github.com/0xPlaygrounds/rig) を基盤とし、強力な3層メモリシステム（短期・中期・長期記憶）を備えた拡張性の高いチャットボットを提供します。

## 主な特徴

- **階層型メモリシステム:**
  - **短期記憶:** 最新の会話コンテキストを DashMap でインメモリ保持
  - **中期記憶:** 過去の会話のサマリー（LLM生成）を Qdrant ベクトル DB で管理・自動クリーンアップ
  - **長期記憶:** ユーザーに関する重要な事実を非同期バックグラウンドで抽出・永続化
- **スラッシュコマンド & プレフィックスコマンド:** `/ask`, `/clear`, `/history` および `w!ask`, `w!clear` に対応
- **Poise フレームワーク:** Serenity 上に構築された堅牢なコマンドルーティング
- **OpenAI 互換プロバイダ:** 任意の OpenAI 互換 API（Crof, OpenRouter など）を LLM・埋め込みモデルとして利用可能
- **ベクトル検索:** Qdrant（本番）および InMemory（テスト用）の2つの Vector DB 実装
- **OpenAI 互換埋め込みモデル:** フォールバックとして疑似ランダム MockEmbedder を搭載
- **非同期長期記憶抽出:** セマフォ制御されたバックグラウンドタスクで LLM 呼び出しを並行処理
- **日次ログローテーション:** `logs/nekoai.log` への日次ログ出力
- **インタラクティブな起動:** 初回起動時に `.config/config.json` の存在確認とセットアップ案内
- **プログレスバー:** 初期化の各段階を視覚的に表示（indicatif）

## 技術スタック

- **[Rig](https://github.com/0xPlaygrounds/rig)**: エージェント構築・LLM 抽象化・埋め込みモデル
- **[Serenity](https://github.com/serenity-rs/serenity)**: Discord API クライアント
- **[Poise](https://github.com/serenity-rs/poise)**: Discord コマンドフレームワーク（Serenity ラッパー）
- **[Qdrant](https://qdrant.tech/)**: ベクトル検索エンジン（gRPC: 6334）
- **[Tokio](https://tokio.rs/)**: 非同期ランタイム
- **[Tracing](https://github.com/tokio-rs/tracing)**: 構造化ログ（ファイル出力 + 日次ローテーション）
- **[DashMap](https://github.com/xacrimon/dashmap)**: 並行 HashMap（短期記憶）
- **[config-rs](https://github.com/mehcode/config-rs)**: 設定ファイルローディング

## ディレクトリ構造

プロジェクトは Cargo Workspace 形式のモノレポで構成されています。詳細は [docs/architecture.md](docs/architecture.md) および [docs/workflow/](docs/workflow/) を参照してください。

```text
NekoAI/
├── Cargo.toml                   # workspace 定義 (7 クレート)
├── justfile                     # タスクランナー
├── rustfmt.toml                 # Rust フォーマット設定
├── .env.example                 # 環境変数テンプレート (LOG_LEVEL)
├── .config/
│   ├── config.json              # 設定ファイル (JSON)
│   └── INSTRUCTION.md           # システムプロンプト（エージェントの振る舞い定義）
├── crates/
│   ├── cli/                     # エントリポイント・CLI 起動・プログレスバー
│   ├── config/                  # 設定スキーマ定義・JSON ロード・SecretKey
│   ├── discord/                 # Serenity + Poise による Discord イベント処理・
│   │                            # コマンドルーティング (/ask, /clear, /history)
│   ├── agent/                   # Rig エージェントの推論ループ・セッション管理・
│   │                            # 記憶想起・コンテキスト構築・長期記憶抽出
│   ├── memory/                  # 3層記憶（短期・中期・長期）+ Vector DB 抽象化
│   │                            # (Qdrant / InMemory) + 埋め込み
│   ├── domain/                  # 共通データ型 (SessionKey, SessionKind)
│   ├── infra/                   # Tracing ログ初期化（日次ローテーション）
│   └── tools/                   # [未実装] ツールレジストリ・実行・権限管理
├── docs/
│   ├── architecture.md          # アーキテクチャ設計書
│   ├── workflow/                # 各クレートのワークフロー詳細
│   └── fix/                     # コードレビュー文書
├── logs/                        # ログ出力先（自動生成）
└── target/                      # ビルド成果物
```

## セットアップ

### 1. 必要条件
- [Rust](https://www.rust-lang.org/)（最新安定版、edition 2024）
- [Qdrant](https://qdrant.tech/)（ベクトル検索を使用する場合）
- [just](https://github.com/casey/just)（タスクランナー）
- Docker（Qdrant コンテナ実行時）

### 2. インストール
```bash
git clone https://github.com/midorin-Linux/NekoAI.git
cd NekoAI
```

### 3. 設定
`.config/config.json` を作成します（初回起動時に対話的なセットアップ案内があります）。

設定項目の例:
```json
{
  "discord": {
    "token": "YOUR_DISCORD_BOT_TOKEN",
    "guild_id": 1234567890
  },
  "provider": {
    "conversation_model": {
      "provider_base_url": "https://api.openai.com/v1",
      "api_key": "YOUR_API_KEY",
      "model_name": "gpt-4o",
      "parameters": {
        "max_token": 262144,
        "temperature": 1.0,
        "top_p": 0.95
      }
    },
    "summarizer_model": {
      "provider_base_url": "https://api.openai.com/v1",
      "api_key": "YOUR_API_KEY",
      "model_name": "gpt-4o",
      "parameters": {
        "max_token": 262144,
        "temperature": 0.2,
        "top_p": 0.95
      }
    },
    "embedding_model": {
      "provider_base_url": "https://api.openai.com/v1",
      "api_key": "YOUR_API_KEY",
      "model_name": "text-embedding-3-small",
      "dimension": 1536
    }
  },
  "memory": {
    "vector_db": {
      "url": "http://localhost:6334",
      "api_key": "",
      "mid_term_collection": "mid_term",
      "long_term_collection": "long_term"
    },
    "short_term_max_entries": 20,
    "mid_term_top_k": 3,
    "long_term_top_k": 5,
    "mid_term_retention_days": 30
  }
}
```

`.config/INSTRUCTION.md` にはシステムプロンプトを記述します（なければデフォルトプロンプトが使用されます）。

環境変数（`.env` ファイル）:
```env
LOG_LEVEL=info   # debug | info | warn | error
```

### 4. Qdrant のセットアップと起動（ベクトル検索を使用する場合）

#### 4a. セットアップ（初回のみ）
Docker イメージのプルと永続ボリュームを作成します。

```bash
just qdrant-setup
```

#### 4b. 起動
```bash
just qdrant-up
```

初回実行時は `qdrant-setup` でイメージのプルとボリューム作成が行われます。2回目以降は `qdrant-up` だけで起動できます。

#### 4c. 停止
```bash
just qdrant-down
```

### 5. 実行
```bash
# 開発用
just neko start
```

## コマンド一覧

### Discord コマンド

| コマンド | 種別 | 説明 |
|---------|------|------|
| `/ask <message>` | スラッシュ / プレフィックス (`w!ask`) | エージェントへメッセージを送信 |
| `/clear` | スラッシュ / プレフィックス (`w!clear`) | 現在のセッション履歴をリセット（中期記憶に昇格） |
| `/history` | スラッシュのみ | 直近の会話履歴を表示 |

### just コマンド

| コマンド | 説明 |
|---------|------|
| `just neko start` | ビルドして実行 |
| `just fmt` | コードフォーマット（nightly 必須） |
| `just qdrant-setup` | Qdrant Docker イメージのプルと永続ボリュームの作成（初回のみ必要） |
| `just qdrant-up` | Qdrant Docker コンテナを起動 |
| `just qdrant-down` | Qdrant Docker コンテナを停止 |

## データフロー

```
Discord メッセージ受信 (/ask, w!ask)
        │
        ▼
SessionKey 解決（SessionKind: GuildChannel / Thread / DirectMessage）
        │
        ▼
AgentRuntime.submit()
  ├─ SessionManager: セッション取得 or 作成
  ├─ MemoryStore.recall(): 中期・長期記憶をベクトル検索
  ├─ ContextManager.build(): システムプロンプト + 記憶 + 履歴を統合
  ├─ OpenAICompatibleAdapter: Rig Agent 構築
  ├─ agent.chat(): LLM 推論実行
  ├─ ShortTermMemory.push_turn(): 短期記憶に保存
  ├─ should_summarize() → promote_to_mid_term(): LLM による要約生成 + ベクトル保存
  ├─ SessionManager.append(): セッション履歴に追加
  └─ spawn_long_term_extraction(): 重要情報を非同期抽出 → LongTermMemory に保存
        │
        ▼
Discord へ応答送信（2000 文字超は自動分割）
```

## 開発

### フォーマットとリンター
```bash
just fmt                            # コードフォーマット
cargo clippy -- -D warnings         # 静的解析
cargo build --bin nekoai-cli        # ビルド
```

### プロジェクト構成の詳細
各クレートの詳細なワークフローと責務は以下のドキュメントを参照してください:

- [アーキテクチャ設計書](docs/architecture.md)
- [CLI ワークフロー](docs/workflow/cli.md)
- [Config ワークフロー](docs/workflow/config.md)
- [Discord ワークフロー](docs/workflow/discord.md)
- [Agent ワークフロー](docs/workflow/agent.md)
- [Memory ワークフロー](docs/workflow/memory.md)
- [Domain ワークフロー](docs/workflow/domain.md)
- [Infra ワークフロー](docs/workflow/infra.md)

## 既知の問題・制限事項

詳細は [BUGS.md](BUGS.md) および [FEATURES.md](FEATURES.md) を参照してください。

主なもの:
- **tools クレート**: 空の状態。ツールレジストリ・MCP クライアントは未実装
- **セットアップウィザード**: TUI ベースのセットアップは未実装（現在は CLI フォールバックのみ）
- **永続化**: セッションはインメモリのみ（SQLite 未実装）
- **ProviderAdapter**: OpenAI 互換のみ。Anthropic 等への切り替えは未対応
- **`history` コマンド**: 2000 文字制限を超えると送信失敗する可能性あり
- **BlockOn 問題**: Vector DB / Embedder が同期 trait で `block_on` を使用（一部環境で panic の可能性）

## 今後の予定

詳細は [FEATURES.md](FEATURES.md) を参照してください。

- セットアップウィザード実装
- ツールシステム（ToolRegistry, MCP クライアント）
- SQLite 永続化
- Web UI（feature = "web-ui"）
- ProviderAdapter のマルチプロバイダ対応
- graceful shutdown
- トークナイザーを使用した自然な分割

## ライセンス
[Apache-2.0](LICENSE)

> Generated by Kimi-K2.6 (管理者による精査済み)
