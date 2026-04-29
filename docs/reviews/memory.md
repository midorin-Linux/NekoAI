# nekoai-memory レビュー

## 対象
- `crates/memory/src/store.rs`
- `crates/memory/src/short_term.rs`
- `crates/memory/src/mid_term.rs`
- `crates/memory/src/long_term.rs`
- `crates/memory/src/embedding.rs`
- `crates/memory/src/vector_db/*.rs`

## 指摘事項

### 1. [High] Qdrant クライアント生成失敗時に `panic` する (完了)
- 該当: `crates/memory/src/store.rs:44`, `crates/memory/src/store.rs:54`
- `MemoryStore::new` で `.expect("failed to create Qdrant client")` を使っており、設定不正時にプロセスが即時終了します。
- 修正案:
  - `MemoryStore::new` を `Result<Self>` に変更して呼び出し元へ返す
  - CLI でユーザー向けに原因（URL/APIキー）を表示する

### 2. [Medium] 埋め込みモデル失敗時に `MockEmbedder` へ自動フォールバックし、品質劣化がサイレントに起きる
- 該当: `crates/memory/src/store.rs:72`, `crates/memory/src/embedding.rs:60`
- 起動は継続できますが、実質ランダム埋め込みとなり検索品質が大きく低下します。
- 修正案:
  - 本番向けには fail-fast（起動失敗）をデフォルトにする
  - 開発時のみフォールバックを許可するフラグを設ける

### 3. [Medium] 長期記憶検索がセッションスコープ固定で、チャネル横断の再利用ができない
- 該当: `crates/memory/src/long_term.rs:131`, `crates/memory/src/vector_db/qdrant.rs:241`
- `guild_id + channel_id + kind` で絞るため、同一ユーザー/同一サーバー内でも別チャネルでは想起されません。
- 修正案:
  - 用途に応じて検索スコープを選べる API（session / guild / user）を `MemoryStore` に公開する
  - `recall` 側で優先順位付きマージ（session > user > guild）を実施する

### 4. [Low] `short_term_max_entries` の偶奇次第で会話ターン境界が崩れる
- 該当: `crates/memory/src/short_term.rs:35`, `crates/memory/src/short_term.rs:63`
- エントリ単位で古い要素を削除するため、奇数設定では `User/Assistant` ペアが崩れる可能性があります。
- 修正案:
  - 設定値を偶数にバリデートする
  - もしくは `turn` 単位で保持・圧縮する構造へ変更する

### 5. [Low] 既存コレクションの次元不一致を検知しない
- 該当: `crates/memory/src/vector_db/qdrant.rs:140`
- `ensure_collection` は「存在確認→未存在なら作成」のみで、既存コレクションの次元検証を行いません。
- 修正案:
  - 既存コレクション情報を取得し、設定次元と不一致ならエラーを返す

### 6. [Medium] `QdrantClient` 作成時の `.expect()` が残存しており修正が不十分（#1 の再発）
- 該当: `crates/memory/src/store.rs:55`
- issue 1 は「完了」とされていますが、`MemoryStore::new` が `Result<Self>` を返すようになった一方で、QdrantClient の作成箇所では依然として `.expect("failed to create Qdrant client")` が使われておりパニックします。
- 修正案:
  - `.expect()` を `?` に変更し、エラーを呼び出し元に伝播させる

### 7. [Low] `start_cleanup_job` の停止機構がなく、シャットダウン時にタスクが制御不能
- 該当: `crates/memory/src/store.rs:237`
- `start_cleanup_job` は `JoinHandle` を返さず、シャットダウン時にバックグラウンドタスクを graceful に停止する手段がありません。
- 修正案:
  - `JoinHandle` を返し、`AgentRuntime::shutdown()` 経由でキャンセルできるようにする
  - もしくはシャットダウンチャネルを追加する

### 8. [Low] `memory` クレートの `Cargo.toml` に `serenity` が依存として含まれているがソースコード上未使用
- 該当: `crates/memory/Cargo.toml:18`
- ソースコードで `serenity` を直接 import しておらず、`nekoai-domain` 経由で推移的に利用されるのみです。
- 修正案:
  - 使用実態がないため依存から削除する
