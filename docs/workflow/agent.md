# `nekoai-agent` クレートのワークフロー

## 役割

`nekoai-agent` は、ユーザー入力を受けて LLM 推論を実行し、セッション管理・記憶連携・応答生成を行う中核レイヤーです。Discord や CLI などの入出力層には依存せず、`SessionKey`・`Option<String>`（user_id）・文字列入力を受けて応答文字列を返します。

## 主な構成

- `runtime.rs`: 起動初期化、推論ループ (AgentRuntime)、要約/長期記憶抽出トリガー、抽出タスクプロセッサ
- `context.rs`: システムプロンプト構築（記憶注入）と会話ターン圧縮
- `session.rs`: セッションの生成・更新・削除（SessionManager）
- `provider.rs`: OpenAI 互換の Rig `AgentBuilder` を組み立てる `OpenAICompatibleAdapter`

## 起動時ワークフロー（`AgentRuntime::new_with_progress`）

合計 5 ステップの進捗 (`RuntimeInitProgress`) を返し、CLI 側のプログレスバーに反映されます。

1. `SessionManager` を `Arc<Mutex<...>>` で初期化
2. `.config/INSTRUCTION.md` を読み込み、システム指示として保持（なければ初期化失敗）
3. `ContextManager` を生成（`max_tokens=16384`, `compaction_threshold=0.7`）。`MemoryStore` を `Arc` でラップして保持
4. OpenAI 互換クライアントを `config.provider.language_model` の設定（`api_key`, `provider_base_url`）から構築し、`OpenAICompatibleAdapter` でラップ
5. モデル名・生成パラメータ（`max_token`, `temperature`, `top_p`）と共に `AgentRuntime` を構築

**内部で実行される追加の初期化**:

- mpsc チャネル（容量 100）を作成し、長期記憶抽出タスクを送信する `extraction_tx` を保持
- `tokio::spawn` で `extraction_task_processor` をバックグラウンド起動（抽出タスクを逐次処理）
- 同時実行制限用の `Semaphore`（最大 3）を作成

`new()` は `new_with_progress` を空のコールバックで呼び出す簡易ラッパーです。

## 推論ワークフロー（`submit`）

`submit(session_key, user_id, user_input) -> Result<AgentResponse>`:

1. `SessionManager` から `SessionKey` 単位でセッション取得（なければ新規作成）
2. `MemoryStore::recall` で中期/長期記憶を検索
3. `ContextManager::build` でプロンプトコンテキストを構築（後述）
4. `OpenAICompatibleAdapter` で Rig エージェントを生成（`preamble` にシステムプロンプトを設定）
5. コンテキストの既存ターンを `chat_history`（`Vec<Message>` の User/Assistant ペア）に変換
6. `agent.chat(user_message, chat_history)` を実行して応答を取得
7. 短期記憶へ追記（`push_short_term`）
8. `should_summarize` が true なら中期記憶への昇格処理を実行
9. セッション履歴へ追記（`SessionManager::append`）
10. `spawn_long_term_extraction` で抽出タスクを mpsc チャネルにキューイング
11. `AgentResponse { content }` を返却

**プロンプト構成**（実際のデータ構造）:
- **System** (`preamble`): ベースシステムプロンプト + 注入された記憶（`<ImportantMemories>` / `<PastConversations>` タグ）
- **Chat history** (`chat_history`): 圧縮済みの過去ターンを User/Assistant `Message` ペアとして配列で渡す
- **Current message**: 最新のユーザー入力を `agent.chat()` の第一引数として個別に渡す

## 中期記憶昇格ワークフロー

`promote_short_term_to_mid_term` は次の順序で動作します。

1. 対象セッションの短期メッセージ一覧を取得
2. 空なら即座に `Ok(())` を返してスキップ
3. `format_short_term_messages` でメッセージを整形
4. 同じモデルで要約用プロンプトを投げ、3文要約を生成（`generate_mid_term_summary`）
5. `MemoryStore::promote_to_mid_term` で要約を保存
6. （短期記憶はクリアしない — 後続の会話で再利用される）

**注意**: 以前の実装とは異なり、昇格後も短期記憶は保持されます。`clear_session` 時のみ明示的に `clear_short_term` が呼ばれます。

### トリガー

- **圧縮閾値到達時**: `submit` の途中で `should_summarize` が true の場合に即時実行
- **`/clear` 実行時（`clear_session`）**: 短期メッセージを `tokio::spawn` で非同期に昇格後、`clear_short_term` + `SessionManager::clear` を実行

## 長期記憶抽出ワークフロー

抽出はバックグラウンドの `extraction_task_processor` で処理されます。

**キューイング**:
1. `submit` 完了後、`spawn_long_term_extraction` が `ExtractionTask { session_key, user_id, response }` を mpsc チャネルに `try_send`
2. キューが満杯の場合は `warn` ログを出力しタスクを破棄（本体応答には影響しない）

**非同期ワーカー（`extraction_task_processor`）**:
3. mpsc チャネルからタスクを受信
4. `Semaphore` で同時実行数を最大 3 に制限
5. 各タスクを `tokio::spawn` で非同期実行

**抽出処理（`extract_and_store_long_term_facts`）**:
6. 応答文から JSON 配列を抽出するための専用プロンプトを送信
7. 応答を `Vec<ExtractedFact>` としてパース（`serde_json`）
8. パース失敗時は文字列中の `[`...`]` 部分で再試行（`parse_extracted_facts`）
9. パースに失敗した場合、`tokio_retry`（最大 1 回リトライ、1 秒間隔）で再実行
10. 空でなければ `MemoryStore::extract_long_term` で保存

保存データは `(fact, tags)` と `user_id`（存在する場合）です。

## セッション操作ワークフロー

- `get_history`: `SessionManager::get` でセッションを取得し `.clone()` して返す
- `clear_session`:
  1. 短期メッセージを取得
  2. 空でなければ `tokio::spawn` で非同期に `generate_mid_term_summary` → `promote_to_mid_term` を実行
  3. `MemoryStore::clear_short_term` で短期記憶を削除
  4. `SessionManager::clear` でセッションを削除
- `shutdown`:
  1. `extraction_tx`（mpsc の Sender）を drop して抽出ワーカーを終了
  2. 2 秒待機して処理中のタスク完了を待つ

## エラー時の挙動

- モデル呼び出しや保存で失敗した場合は `Result::Err` を返却
- 中期昇格/長期抽出は失敗しても本体応答は継続し、`warn` ログで通知
- `.config/INSTRUCTION.md` がない場合は初期化失敗（`context` 付き `Err`）
- 長期記憶抽出の JSON パース失敗時は `tokio_retry`（最大 1 回）で再試行、それでも失敗した場合は `warn` ログ
- 抽出キューが満杯の場合はタスクを破棄し `warn` ログで通知

## 連携ポイント

- 入力: `nekoai-discord`（`ask` コマンド）
- 設定: `nekoai-config`（モデル/API/パラメータ）
- 記憶: `nekoai-memory`
- 型: `nekoai-domain::agent::session::SessionKey`
