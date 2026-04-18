ARCHITECTURE.md に記載されている機能について、`memory` / `agent` / `discord` の実装を照合し、実装状況と潜在的な問題点をレポートします。

## 1. 実装状況サマリ

| カテゴリ | 機能 | 実装状況 | 備考 |
|---|---|---|---|
| Discord 層 | EventHandler (ready) | ✅ | `handler.rs` |
| Discord 層 | メッセージメンション応答 (`message` handler) | ❌ 未実装 | `EventHandler::message` がなく、通常発話での起動不可 |
| Discord 層 | CommandRouter (poise Framework) | ✅ | |
| Discord 層 | `/ask` | ✅ | |
| Discord 層 | `/clear` | ✅ | |
| Discord 層 | `/history` | ✅ | |
| Discord 層 | `/tools` | ❌ 未実装 | |
| Discord 層 | `/abort` | ❌ 未実装 | `AbortHandle` そのものが未実装 |
| Discord 層 | `/memory list` / `forget` / `stats` / `clear-session` | ❌ 未実装 | |
| Discord 層 | ContextResolver → SessionKey | ✅ | `session_resolver.rs` |
| Discord 層 | typing indicator | △ 部分的 | prefix のみ。slash は `defer()` で代替 |
| Discord 層 | メッセージ分割送信 (2000 文字) | ✅ | `split_message` |
| Agent 層 | AgentRuntime | ✅ | |
| Agent 層 | SessionManager | ✅ | |
| Agent 層 | ContextManager (圧縮・記憶注入) | △ 部分的 | 圧縮は「turn 数ベース」で簡易。compaction_threshold は未使用 |
| Agent 層 | ProviderAdapter (Anthropic/OpenAI 切替) | ❌ 簡略化 | OpenAI 互換のみ。`trait ProviderAdapter` 無し |
| Agent 層 | 長期記憶の非同期抽出 | ✅ | `spawn_long_term_extraction` |
| Agent 層 | 中期記憶への昇格 (should_summarize / clear トリガー) | ✅ | |
| Memory 層 | MemoryStore 統合 API | ✅ | |
| Memory 層 | ShortTermMemory (DashMap) | ✅ | |
| Memory 層 | MidTermMemory (Qdrant) | ✅ | |
| Memory 層 | LongTermMemory (Qdrant) | ✅ | |
| Memory 層 | Embedder (Rig / Mock fallback) | ✅ | |
| Memory 層 | VectorDbClient trait + Qdrant + InMemory | ✅ | |
| Tool 層 | ToolRegistry / ビルトインツール | ❌ 未実装 | `tools` クレート自体が無い想定 |
| Tool 層 | MCP クライアント (stdio/sse) | ❌ 未実装 | |
| Tool 層 | PermissionGuard | ❌ 未実装 | |
| Infra 層 | SQLite 永続化 (sessions/messages/tool_executions) | ❌ 未実装 | セッションはインメモリのみ |
| Infra 層 | EventBus (broadcast) | ❌ 未実装 | `MemoryRecalled` 等のイベント通知なし |
| Infra 層 | HttpServer / Axum (feature: web-ui) | ❌ 未実装 | |
| Infra 層 | Metrics | ❌ 未実装 | tracing のみ |
| CLI 層 | `run` / `config` / `mcp-server` サブコマンド | 未確認 | 今回 `cli` クレートは未参照 |
| Setup 層 | ratatui ウィザード | 未確認 | 今回 `setup` クレートは未参照 |

---

## 2. バグ・正常に動作しない/危険なポイント

### 🔴 致命的：Qdrant / Embedder の非同期呼び出しでデッドロックの恐れ

`memory/src/vector_db/qdrant.rs` と `memory/src/embedding.rs` は同じパターンを使っています。

```rust
fn run_async<T, E>(future: impl Future<Output = Result<T, E>>) -> anyhow::Result<T> {
    let result = if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(future))
    } else {
        // ...
    };
}
```


これは `VectorDbClient` / `Embedder` を同期 trait にしてしまったために、内部で `block_on` を呼び出している実装です。問題点：

1. **`block_in_place` は `current_thread` ランタイムでは panic します**。`tokio::main` で指定なしだと multi-thread なので動くものの、シングルスレッド設定やテストでは破綻します。
2. `AgentRuntime::submit` は async で呼ばれているのに、中で同期メソッド `memory_store.recall()` を呼び、その中で `block_in_place + block_on` してネストします。無駄にスレッドを塞ぎ、スループットが低下します。

**推奨**：`VectorDbClient` と `Embedder` を `async_trait` 化し、素直に `.await` させるべきです。同じく `MemoryStore::recall`, `push_short_term`, `promote_to_mid_term`, `extract_long_term` も async に。

### 🔴 致命的：`InMemoryVectorDb` でデッドロック

```rust
fn upsert(&self, req: UpsertRequest<'_>) -> anyhow::Result<()> {
    let mut collections = self.collections.blocking_write();
    // ...
}
```


`tokio::sync::RwLock` の `blocking_write` は、**Tokio ランタイムのワーカースレッド上で呼ぶと panic します**（"Cannot block the current thread from within a runtime"）。テストや `InMemoryVectorDb` 経路で実質使えません。`std::sync::RwLock` に切り替えるのが素直です。

### 🟠 中期記憶のフィルタが厳しすぎて、ほぼヒットしない

`mid_term.rs` / `long_term.rs` の `session_scope_filter` は `guild_id + channel_id + kind` すべて一致を要求しています。しかし architecture.md は「guild_id + channel_id でフィルタ」としており、**スレッドの場合はセッションキーに `thread_id` が入り、`channel_id` が親チャンネル ID になる**ため、スレッド記憶と通常発話の記憶は分離されます（ここは仕様として OK）。一方で、

- **長期記憶は `guild_id + channel_id` 単位でしか検索されない**ので、ユーザー個人の好みなどサーバー横断で参照すべき情報が想起されません。architecture.md の「ユーザーの好み・重要な事実」という意図とズレがあります。最低でも `guild_id` のみのフィルタ or `user_id` 基準の検索方法を別途用意すべきです。
- ペイロードに `user_id` が保存されていないため、後からユーザー単位の記憶検索に切り替えられません。

### 🟠 `SessionManager::get` が `&mut self` を要求し、`get_history` が `&self` で使えてしまっている

```rust
pub fn get_history(&self, session_key: &SessionKey) -> Result<Session> {
    let mut session_manager = self.session_manager.lock().expect(...);
    session_manager.get(session_key).map(|s| s.clone())
}
```


`Mutex` 越しなので動作はしますが、`get` が `&mut self` なのは設計上おかしく、本来は `&self` で参照取得可能にすべきです。副作用が無いのにロックを取得する必要があり、並行性も落ちます。

### 🟠 `session_manager` が `std::sync::Mutex` で保持されている

```rust
session_manager: Arc<Mutex<SessionManager>>,
```


- async コンテキストで `std::sync::Mutex` を握ったまま `.await` すると危険です。実際 `submit` 内では lock → clone → drop → `.await` のパターンで逃げていますが、設計としては `tokio::sync::Mutex` または `DashMap` に寄せるのが無難です。
- ARCHITECTURE には「`DashMap` または `Arc<Mutex>` のどちらでもよい」という記述があるのでルール違反ではないものの、他の DashMap 使用箇所と揃えるのが一貫性の観点で良いです。

### 🟠 ContextManager の「圧縮」実装が雑

```rust
let max_turns = (self.max_tokens / 512).max(1);
if turns.len() > max_turns {
    ...
}
```


- **ハードコードされた `max_tokens = 16384` と `0.7`** が `AgentRuntime::new_with_progress` にベタ書き（設定ファイルから読めていない）。`config.session.max_history_messages`、`compaction_threshold` が無視されています。
- `compaction_threshold: f32` は `#[allow(dead_code)]` で完全に未使用。
- architecture では「トークン数で判定し古い部分をサマリーに置換」する設計ですが、実際は単純な drain で、中期記憶への反映もここでは行われていません。

### 🟠 プロンプト構築が `format!` ベタ書きで Rig の Message を活かしていない

```rust
let mut prompt_text = String::new();
prompt_text.push_str(&format!("System: {}\n\n", context.system_prompt));
for turn in &context.turns {
    prompt_text.push_str(&format!("User: {}\nAssistant: {}\n", ...));
}
prompt_text.push_str(&format!("User: {}", context.user_message));

let result = agent.prompt(prompt_text).await?;
```


- `session.messages: VecDeque<Message>` を保持しているのに、実際の prompt には使っていません。
- `AgentBuilder` の `.preamble(system_prompt)` や `.chat(history, prompt)` のような正規の API を使わず、全部を単一の `prompt` 文字列にしているため、ロール情報がモデルに正しく伝わらず、システムプロンプトも User メッセージとして扱われる恐れがあります。
- architecture 8.4 が示す「`preamble(context.system_prompt())`」の意図と乖離しています。

### 🟠 `AgentRuntime::submit` の最後の `append` がダブルカウントになりかねない

```rust
self.memory_store.push_short_term(&session_key, &user_input, result.as_str());
// ...
session_manager.append(&session_key, &user_input, result.as_str());
```


短期記憶（要約昇格用）とセッション履歴（プロンプトに乗る履歴）が別々に二重管理されています。どちらも圧縮ロジックを別々に持っているため、

- ShortTerm の max_entry を超えたら `should_summarize` で中期昇格 → ShortTerm クリア
- しかし SessionManager の turns はクリアされない

という非対称があり、中期要約後も `context.turns` に同じ内容が残って LLM に渡るので、「サマリー + 生履歴」で冗長なプロンプトになります。ARCHITECTURE 的には SessionManager.messages は「短期記憶の別表現」であり、同期するか一本化すべきです。

### 🟠 `extract_long_term` の抽出プロンプトが常に呼ばれる

`spawn_long_term_extraction` は **毎ターン** LLM 呼び出しを行います。トークンコストと API レート制限的に重いので、

- 短期記憶が一定量たまってからまとめて抽出
- あるいは特定フラグ（config の `extract_facts`）で制御

すべきですが、`config.memory.extract_facts` を参照していません。

### 🟠 長期記憶抽出プロンプトの入力が `response` のみ

```rust
let prompt = format!(
    "以下の応答から、...\n応答: {}",
    response
);
```


ユーザー入力が渡されないため、「アシスタントの応答だけから将来必要な事実を推測」することになり、文脈が無くて精度が出にくいです。`user_input` も含めるべきです。

### 🟠 `ask` コマンドで `global_name` を `unwrap_or_else` している

```rust
ctx.author().global_name.clone().unwrap_or_else(|| ctx.author().name.clone())
```


動作はしますが、ログ系以外での表示にはニックネーム（ギルド member nick）を優先するのが UX 的に親切です。軽微。

### 🟠 `session_resolver` が `guild_id` 引数を使っていない

`guild_id` を引数で受け取っているものの、`to_channel` の失敗時フォールバックでしか使っていません。`Channel::Guild` のときに `guild_id` を突き合わせてもよいですが、実害はほぼ無い（ただし未使用パラメータに近い）。

### 🟠 `history` コマンドが 2000 文字超で失敗する

```rust
let messages = history.turns.iter().map(...).join("\n\n");
ctx.say(messages).await?;
```


`ask` では `split_message` しているのに、`history` は分割していません。ターン数が多いと Discord の 2000 文字制限に引っかかり、エラー送信失敗します。

### 🟠 Qdrant `delete_by_filter` の戻り値が不正確

```rust
let count_response = ... client.count(... exact(true)).await ...;
let deleted = count_response.result.map(|r| r.count).unwrap_or(0);
if deleted == 0 { return Ok(0); }
let delete_builder = ... .points(qdrant_filter) ...;
```


`count` と `delete` の間に別プロセスが upsert/delete する可能性があり、「返した deleted 数」と「実際に消えた数」が一致しない可能性があります。また `count` + `delete` の 2 ラウンドトリップは無駄なので、`delete` のレスポンスから件数を取る方式のほうが正確です。

### 🟠 `DiscordClient::new` で生成した `_http` / `_shared_cache` が未使用

```rust
let _http = Arc::new(serenity::all::Http::new(&discord_token));
let _shared_cache = Arc::new(serenity::all::Cache::new());
```


使っていない Arc を生成してすぐ捨てているので、意味のない処理が残っています。削除するか、`Client::builder` に渡すべきです。

### 🟠 intents が過剰

`GUILD_PRESENCES` / `GUILD_MEMBERS` / `GUILD_VOICE_STATES` は現行機能では不要で、いずれも **Privileged Intent** を含みます。ダッシュボードで有効化していないと接続自体が失敗する、もしくは警告が出ます。`GUILDS | GUILD_MESSAGES | MESSAGE_CONTENT | DIRECT_MESSAGES` 程度で足ります。DM 対応したいなら `DIRECT_MESSAGES` は逆に必須です（現在欠落）。

### 🟠 DM サポートが事実上無い

- `session_resolver` は DM を `SessionKind::DirectMessage` にマッピングしていますが、intents に `DIRECT_MESSAGES` が含まれておらず、`message` handler も未実装なので、DM からの通常発話は処理できません。slash コマンドは拾えるものの、ARCHITECTURE の「DM はユーザーごとに独立したセッション」は実質機能しません。

### 🟠 `Session` に `user_id` 情報がなく、DM の「ユーザーごと」分離が曖昧

DM の `SessionKey` は `channel_id` のみで分離されます。Discord の DM は 1 ユーザー 1 チャンネルなので偶然機能しますが、`SessionKind::DirectMessage` のとき guild_id=None + channel_id で十分というのは実装の暗黙の前提です。ドキュメント化されていないので将来の変更で壊れやすいです。

### 🟠 エラーログの user 向け表示

```rust
Err(err) => {
    error!(error = %err, "failed to generate agent response");
    err.to_string()   // ← そのままユーザーに送信
}
```


LLM プロバイダのエラー（API キー・レート制限・内部例外）がユーザーに生表示されます。情報漏洩リスクがあるため、ユーザー向けには汎用メッセージ、ログには詳細、という分離が望ましいです。

### 🟡 軽微：`chrono` の廃止 API

`DateTime::from_timestamp(ts, 0).unwrap_or_default()` は OK ですが、`unwrap_or_default()` が `DateTime<Utc>::default()` = Unix epoch になるので、「タイムスタンプ取得失敗で 1970 年と区別つかない」事象が起きえます。`Option<DateTime<Utc>>` のままメタデータに載せるほうが正直です。

### 🟡 軽微：`ContextManager::compaction_threshold` フィールドが未使用のまま

実装する意図があるなら使う、無いなら削除。`#[allow(dead_code)]` で黙らせているのは設計書との乖離を隠してしまっています。

---

## 3. ARCHITECTURE.md との主要な乖離まとめ

1. **Tool System 層が丸ごと未実装**（registry / permission / MCP / builtin tools / abort）
2. **Infrastructure 層が未実装**（SQLite, EventBus, metrics, web-ui）
3. **Discord の通常メッセージ応答 (`message` handler)** が無く、slash / prefix コマンド経由のみ
4. **ProviderAdapter trait による Anthropic/OpenAI 切替**が無く、OpenAI 互換のみ
5. **SessionManager の永続化**が無く、プロセス再起動で会話消失
6. **メモリの同期 trait 化** による設計上のアンチパターン（block_on）
7. **`/tools` `/abort` `/memory *`** コマンドが未実装
8. **セッションの自動アーカイブ (`archive_after_minutes`)** が未実装
9. **`config.memory.extract_facts` / `mid_term_retention_days` の自動クリーンアップジョブ** が未起動（`delete_old_entries` は実装されているが誰も呼ばない）
10. **EventBus 経由のイベント通知** が一切無いため、Web UI 拡張の下地が未整備

---

## 4. 改善優先度順の推奨アクション

1. **`VectorDbClient` / `Embedder` / `MemoryStore` の async 化**（`async_trait` 導入）— 今の block_on はランタイム設定次第で即 panic するため最優先。
2. **`InMemoryVectorDb` の `tokio::sync::RwLock` を `std::sync::RwLock` に変更**。
3. **Discord `EventHandler::message` の実装**（メンション検知 + `AgentRuntime.submit` 委譲）と intents の整理（`DIRECT_MESSAGES` 追加、Privileged 系を削除）。
4. **プロンプト構築を Rig の `.preamble()` + `.chat(history, prompt)` に修正**。システム/ユーザー/アシスタントのロールを保つ。
5. **`history` コマンドも `split_message` を通す**（小さな修正）。
6. **`config.session.*` と `config.memory.extract_facts` の反映**（ハードコード削除）。
7. **中期記憶昇格時に SessionManager.turns も同期クリア**（二重会話防止）。
8. **EventBus (tokio::sync::broadcast) の最低限導入** — 後続の Web UI / ツール実装が楽になる。
9. **ToolRegistry + PermissionGuard のスケルトン**だけでも先に置く（Rig の Tool trait を使うだけのダミー）。
10. **SQLite 永続化**（`infra` クレート新設、`sessions` / `messages` の最小スキーマから）。

必要であれば、上記のうち具体的にどれかの修正パッチ（例えば `EventHandler::message` の実装や `history` の分割送信修正、MemoryStore の async 化リファクタリング）を作成しますので、着手したい項目を指定してください。

> Generated by Claude 4.7 Opus