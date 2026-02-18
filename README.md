# NekoAI
これはRustで書かれている**Discord用AIエージェント**です。**OpenAI互換API**を使用して動作させることができ、ツールが組み込まれているため個人でも扱いやすいアプリケーションとなっています。

## 技術スタック
このプロジェクトは主に以下のクレートが使われています。
- **serenity**: [serenity-rs/serenity](https://github.com/serenity-rs/serenity) - A Rust library for the Discord API.
- **poise**: [serenity-rs/poise](https://github.com/serenity-rs/poise) - Discord bot command framework for serenity, with advanced features like edit tracking and flexible argument parsing
- **rig**: [0xPlaygrounds/rig](https://github.com/0xPlaygrounds/rig) - ⚙️🦀 Build modular and scalable LLM Applications in Rust
