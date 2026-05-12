# NekoAI

NekoAI は Rust で開発されている **Discord 用 AI エージェント** です。
[Rig SDK](https://github.com/0xPlaygrounds/rig) を基盤とし、強力な3層メモリシステム（短期・中期・長期記憶）と豊富な Discord ツール群を備えた拡張性の高いチャットボットを提供します。

## 主な特徴

- **階層型メモリシステム:**
  - **短期記憶:** 最新の会話コンテキストを DashMap でインメモリ保持
  - **中期記憶:** 過去の会話のサマリー（LLM生成）を Qdrant ベクトル DB で管理・自動クリーンアップ
  - **長期記憶:** ユーザーに関する重要な事実を非同期バックグラウンドで抽出・永続化
- **豊富な Discord ツール:** メッセージ・チャンネル・ロール・メンバー・スレッド・ボイス・絵文字・招待・スケジュールイベントなど 50 以上のツールを実装。エージェントが Discord サーバーを自律的に操作可能
- **スラッシュコマンド & プレフィックスコマンド:** `/ask`, `/clear`, `/history` および `w!ask`, `w!clear` に対応
- **Poise フレームワーク:** Serenity 上に構築された堅牢なコマンドルーティング
- **OpenAI 互換プロバイダ:** 任意の OpenAI 互換 API（OpenRouter など）を LLM・埋め込みモデルとして利用可能
- **ベクトル検索:** Qdrant（本番）および InMemory（テスト用）の2つの Vector DB 実装
- **非同期長期記憶抽出:** セマフォ制御されたバックグラウンドタスクで LLM 呼び出しを並行処理
- **日次ログローテーション:** `logs/nekoai.log` への日次ログ出力
- **インタラクティブなセットアップウィザード:** dialoguer ベースの4ステップウィザードで初回設定を案内。`DISCORD_AGENT_TOKEN` 環境変数による CLI フォールバックも対応
- **プログレスバー:** 初期化の各段階を視覚的に表示（indicatif）
- **Web UI:** React Router + TailwindCSS + Radix UI によるダッシュボード（`nekoai-gui/`）

## 技術スタック

### Rust バックエンド（`nekoai-rs/`）

- **[Rig](https://github.com/0xPlaygrounds/rig)**: エージェント構築・LLM 抽象化・埋め込みモデル
- **[Serenity](https://github.com/serenity-rs/serenity)**: Discord API クライアント
- **[Poise](https://github.com/serenity-rs/poise)**: Discord コマンドフレームワーク（Serenity ラッパー）
- **[Qdrant](https://qdrant.tech/)**: ベクトル検索エンジン（gRPC: 6334）
- **[Tokio](https://tokio.rs/)**: 非同期ランタイム
- **[Tracing](https://github.com/tokio-rs/tracing)**: 構造化ログ（ファイル出力 + 日次ローテーション）
- **[DashMap](https://github.com/xacrimon/dashmap)**: 並行 HashMap（短期記憶）
- **[dialoguer](https://github.com/console-rs/dialoguer)**: インタラクティブ CLI（セットアップウィザード）
- **[config-rs](https://github.com/mehcode/config-rs)**: 設定ファイルローディング

### Web UI（`nekoai-gui/`）

- **[React Router v7](https://reactrouter.com/)**: フルスタック SSR フレームワーク
- **[TailwindCSS v4](https://tailwindcss.com/)**: ユーティリティファーストCSS
- **[Radix UI Themes](https://www.radix-ui.com/themes)**: UIコンポーネント

## ディレクトリ構造

プロジェクトは Rust バックエンド（`nekoai-rs/`）と Web UI（`nekoai-gui/`）を含むモノレポです。詳細は [docs/architecture.md](docs/architecture.md) および [docs/workflow/](docs/workflow/) を参照してください。

```text
NekoAI/
├── justfile                     # タスクランナー（Windows PowerShell 向け）
├── nekoai-rs/                   # Rust バックエンド（Cargo Workspace）
│   ├── Cargo.toml               # workspace 定義（9 クレート）
│   ├── justfile                 # Rust 用タスクランナー（neko, fmt）
│   ├── rustfmt.toml             # Rust フォーマット設定（nightly 必須）
│   ├── .config/
│   │   ├── config.json.example  # 設定ファイルのテンプレート
│   │   └── INSTRUCTION.md       # システムプロンプト（エージェントの振る舞い定義）
│   ├── cli/                     # エントリポイント・CLI 起動・プログレスバー
│   ├── config/                  # 設定スキーマ定義・JSON ロード・SecretKey
│   ├── discord/                 # Serenity + Poise による Discord イベント処理・
│   │                            # コマンドルーティング (/ask, /clear, /history)
│   ├── agent/                   # Rig エージェントの推論ループ・セッション管理・
│   │                            # 記憶想起・コンテキスト構築・長期記憶抽出
│   ├── memory/                  # 3層記憶（短期・中期・長期）+ Vector DB 抽象化
│   │                            # (Qdrant / InMemory) + 埋め込み
│   ├── setup/                   # 初回セットアップウィザード（dialoguer）・CLI フォールバック
│   ├── domain/                  # 共通データ型 (SessionKey, SessionKind)
│   ├── infra/                   # Tracing ログ初期化（日次ローテーション）
│   └── tools/                   # Discord API ツール群（50+ ツール）
│                                # channel / emoji / guild / invite / member /
│                                # message / role / schedule / thread / voice
├── nekoai-gui/                  # Web UI（React Router + TailwindCSS）
│   ├── app/                     # ルート・コンポーネント
│   └── public/                  # 静的ファイル
├── docs/
│   ├── architecture.md          # アーキテクチャ設計書
│   └── workflow/                # 各クレートのワークフロー詳細
└── .env.example                 # 環境変数テンプレート (LOG_LEVEL)
```

## セットアップ

### 1. 必要条件
- [Rust](https://www.rust-lang.org/)（最新安定版 + nightly）：nightly は `rustfmt.toml` で nightly 専用フォーマットオプションを使用しているため必要
- [Qdrant](https://qdrant.tech/)（ベクトル検索を使用する場合）
- [just](https://github.com/casey/just)（タスクランナー）
- Docker（Qdrant コンテナ実行時）
- **Windows ユーザー:** ルートの `justfile`（Qdrant 管理コマンド）は PowerShell 専用です。Linux / macOS では `docker` コマンドを直接使用してください（[セクション 4](#4-qdrant-のセットアップと起動ベクトル検索を使用する場合) 参照）

### 2. インストール
```bash
git clone https://github.com/midorin-Linux/NekoAI.git
cd NekoAI/nekoai-rs
```

### 3. 設定
`nekoai-rs/.config/config.json` を作成します（初回起動時に dialoguer ベースのインタラクティブなセットアップウィザードが起動します）。

設定項目の例（`nekoai-rs/.config/config.json.example` も参照）:
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

`nekoai-rs/.config/INSTRUCTION.md` にはシステムプロンプトを記述します（なければデフォルトプロンプトが使用されます）。

環境変数（`.env` ファイル、`nekoai-rs/` 直下に作成）:
```env
LOG_LEVEL=info   # debug | info | warn | error
```

または `DISCORD_AGENT_TOKEN` 環境変数を設定することで、セットアップウィザードをスキップできます:
```env
DISCORD_AGENT_TOKEN=YOUR_DISCORD_BOT_TOKEN
```

### 4. Qdrant のセットアップと起動（ベクトル検索を使用する場合）

> **注意:** ルートの `justfile` は Windows PowerShell 向けです。Linux / macOS の場合は `docker` コマンドを直接実行してください。

#### 4a. セットアップ（初回のみ）
```bash
# Windows (PowerShell)
just qdrant-setup

# Linux / macOS
docker pull qdrant/qdrant:latest
docker volume create qdrant_data
```

#### 4b. 起動
```bash
# Windows (PowerShell)
just qdrant-up

# Linux / macOS
docker run -d --name qdrant -p 6333:6333 -p 6334:6334 \
  -e QDRANT__SERVICE__GRPC_PORT="6334" \
  -v qdrant_data:/qdrant/storage qdrant/qdrant:latest
```

#### 4c. 停止
```bash
# Windows (PowerShell)
just qdrant-down

# Linux / macOS
docker stop qdrant
```

### 5. 実行（`nekoai-rs/` ディレクトリ内で実行）
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

### just コマンド（`nekoai-rs/` ディレクトリ内で実行）

| コマンド | 説明 |
|---------|------|
| `just neko start` | ビルドして実行 |
| `just fmt` | コードフォーマット（nightly 必須） |

### just コマンド（ルートディレクトリ、Windows PowerShell のみ）

| コマンド | 説明 |
|---------|------|
| `just qdrant-setup` | Qdrant Docker イメージのプルと永続ボリュームの作成（初回のみ必要） |
| `just qdrant-up` | Qdrant Docker コンテナを起動 |
| `just qdrant-down` | Qdrant Docker コンテナを停止 |

## Discord ツール一覧

エージェントが自律的に Discord サーバーを操作するための 50 以上のツールが実装されています。詳細は [Tools ワークフロー](docs/workflow/tools.md) を参照してください。

| カテゴリ | 主なツール |
|---------|-----------|
| **message** | メッセージ送信・編集・削除・検索・一括削除・ピン・リアクション・Webhook・投票作成・LLM向け履歴取得 |
| **channel** | チャンネル一覧・作成・更新・アーカイブ・権限設定 |
| **role** | ロール一覧・作成/更新・付与/剥奪・並び替え・メンバー一覧・一括操作 |
| **member** | メンバー検索・情報取得・Kick・タイムアウト・ロール操作・調査 |
| **thread** | スレッド作成・一覧・アーカイブ/ロック・メンバー管理 |
| **voice** | ボイス状態一覧・メンバー移動・ミュート/デフ・ステージ管理 |
| **guild** | ギルド情報・設定更新・監査ログ・BAN管理 |
| **invite** | 招待一覧・作成・削除 |
| **emoji** | 絵文字一覧・追加・削除・リアクション統計 |
| **schedule** | イベント一覧・作成・更新/キャンセル・参加者一覧 |

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
  ├─ OpenAICompatibleAdapter: Rig Agent 構築（登録済みツールを注入）
  ├─ agent.chat(): LLM 推論実行（必要に応じてツールを呼び出し）
  ├─ ShortTermMemory.push_turn(): 短期記憶に保存
  ├─ should_summarize() → promote_to_mid_term(): LLM による要約生成 + ベクトル保存
  ├─ SessionManager.append(): セッション履歴に追加
  └─ spawn_long_term_extraction(): 重要情報を非同期抽出 → LongTermMemory に保存
        │
        ▼
Discord へ応答送信（2000 文字超は自動分割）
```

## 開発

### フォーマットとリンター（`nekoai-rs/` ディレクトリ内で実行）
```bash
just fmt                            # コードフォーマット（nightly 必須: rustfmt.toml で nightly 機能を使用）
cargo clippy -- -D warnings         # 静的解析
cargo build --bin nekoai-cli        # ビルド
```

### Web UI の開発（`nekoai-gui/` ディレクトリ内で実行）
```bash
npm install        # 依存関係のインストール
npm run dev        # 開発サーバー起動（http://localhost:5173）
npm run build      # プロダクションビルド
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
- [Tools ワークフロー](docs/workflow/tools.md)

## 既知の問題・制限事項

- **永続化**: セッションはインメモリのみ（SQLite 未実装）
- **ProviderAdapter**: OpenAI 互換のみ。Anthropic 等への切り替えは未対応
- **`history` コマンド**: 2000 文字制限を超えると送信失敗する可能性あり
- **BlockOn 問題**: Vector DB / Embedder が同期 trait で `block_on` を使用（一部環境で panic の可能性）
- **justfile**: ルートの `justfile` は Windows PowerShell 専用。Linux / macOS では `nekoai-rs/justfile` を使用するか、コマンドを直接実行してください

## 今後の予定

- SQLite 永続化
- ProviderAdapter のマルチプロバイダ対応（Anthropic 等）
- graceful shutdown
- トークナイザーを使用した自然なメッセージ分割
- MCP クライアント統合

## ライセンス
[Apache-2.0](LICENSE)
