# NekoAI
これはRustで書かれている**Discord用AIエージェント**です。**OpenAI互換API**を使用して動作させることができ、ツールが組み込まれているため個人でも扱いやすいアプリケーションとなっています。開発ではOpenRouterを使用しているため、OpenRouterを使用することを推奨します。

## 技術スタック
このプロジェクトは主に以下のクレートが使われています。
- **serenity**: [serenity-rs/serenity](https://github.com/serenity-rs/serenity) - A Rust library for the Discord API.
- **poise**: [serenity-rs/poise](https://github.com/serenity-rs/poise) - Discord bot command framework for serenity, with advanced features like edit tracking and flexible argument parsing
- **rig**: [0xPlaygrounds/rig](https://github.com/0xPlaygrounds/rig) - ⚙️🦀 Build modular and scalable LLM Applications in Rust

## 環境構築、使用方法
- **プロジェクトの初期設定**
```bash
git clone https://github.com/midorin-Linux/NekoAI.git
cd NekoAI
cp .env.example .env
```
.envの設定とconfig/settings.tomlの設定をしてください。  
- **Qdrantの設定**
```bash
docker pull qdrant/qdrant
docker run -p 6333:6333 -p 6334:6334 -e QDRANT__SERVICE__GRPC_PORT="6334" qdrant/qdrant
```
Qdrantの起動を確認したらアプリケーションを起動してください。  
- **アプリケーションの起動(本番環境用)**
```bash
cargo run --release
```

## ディレクトリ構造(一部省略)
```text
NekoAI/
├── Cargo.lock
├── Cargo.toml
├── INSTRUCTION.md                          # システムプロンプト
├── README.md
├── config/
│   └── settings.toml                   # 環境非依存の設定
├── docs/                                   # ドキュメント(追加予定)
└── src
    ├── application/                    # アプリケーション層
    │   ├── chat/
    │   │   ├── chat_service.rs             # 通常チャットのユースケース
    │   │   └── mod.rs
    │   ├── command/
    │   │   ├── command_registry.rs         # コマンド登録処理
    │   │   ├── handlers/                   # コマンドハンドラー
    │   │   └── mod.rs
    │   ├── mod.rs
    │   └── traits/
    │       ├── ai_client.rs                # AIクライアントトレイト
    │       └── mod.rs
    ├── domain/
    │   └── mod.rs
    ├── infrastructure/                 # インフラ層
    │   ├── ai/
    │   │   ├── mod.rs
    │   │   ├── rig_client.rs               # Rig SDK ラッパー
    │   │   └── tools/                      # AIエージェント用ツール
    │   ├── discord/
    │   │   ├── client.rs                   # Discordクライアント
    │   │   └── mod.rs
    │   ├── mod.rs
    │   └── store/
    │       ├── in_memory_store.rs          # 短期記憶用のインメモリストア
    │       ├── mod.rs
    │       └── vector_store.rs             # 長期記憶、中期記憶用のベクトルストア
    ├── lib.rs                              # クレートルート
    ├── main.rs                             # エントリーポイント
    ├── models/                         # データ型
    │   ├── error.rs
    │   ├── memory.rs                       # コンテキスト用データ型
    │   └── mod.rs
    ├── presentation/                   # プレゼンテーション層
    │   ├── events/
    │   │   ├── message_handler.rs          # メッセージ受信ハンドラー
    │   │   ├── mod.rs
    │   │   └── ready_handler.rs            # 準備完了ハンドラー
    │   ├── handler.rs
    │   └── mod.rs
    └── shared/
        ├── config.rs                       # 設定ファイルの読み込み
        ├── logger.rs                       # ロギング設定
        └── mod.rs
```
