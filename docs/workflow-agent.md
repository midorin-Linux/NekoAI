# `nekoai-agent` クレートのワークフロー

## 役割

`nekoai-agent` は、ユーザー入力を受けて LLM 推論を実行し、セッション管理・記憶連携・応答生成を行う中核レイヤーです。Discord や CLI などの入出力層には依存せず、`SessionKey` と文字列入力を受けて応答文字列を返します。

## 主な構成

- `runtime.rs`: 起動初期化、推論ループ、要約/長期記憶抽出トリガー
- `context.rs`: システムプロンプト構築と会話ターン圧縮
- `session.rs`: セッションの生成・更新・削除
- `provider.rs`: OpenAI 互換の Rig `AgentBuilder` を組み立て

## 起動時ワークフロー（`AgentRuntime::new_with_progress`）

1. `SessionManager` を `Arc<Mutex<...>>` で初期化
2. `.config/INSTRUCTION.md` を読み込み、システム指示として保持
3. `ContextManager` を生成（`max_tokens=16384`, `compaction_threshold=0.7`）
4. OpenAI 互換クライアントを `provider.language_model` 設定から構築
5. モデル名・生成パラメータ（`max_token`, `temperature`, `top_p`）を保持して初期化完了

`RuntimeInitProgress` は合計 5 ステップで進捗を返し、CLI 側のプログレスバーに反映されます。

## 推論ワークフロー（`submit`）

1. `SessionManager` から `SessionKey` 単位でセッション取得（なければ新規作成）
2. `MemoryStore::recall` で中期/長期記憶を検索
3. `ContextManager::build` でプロンプトコンテキストを構築
4. `OpenAICompatibleAdapter` で Rig エージェントを生成
5. 次の形式で最終プロンプト文字列を構成
   - `System: ...`
   - 既存ターンの `User / Assistant` ペア
   - 最新 `User: ...`
6. `agent.prompt(...)` を実行して応答を取得
7. 短期記憶へ追記（`push_short_term`）
8. 短期記憶件数が閾値到達なら中期記憶への昇格処理を実行
9. セッション履歴へ追記（`SessionManager::append`）
10. 長期記憶抽出タスクを `tokio::spawn` で非同期起動
11. `AgentResponse { content }` を返却

## 中期記憶昇格ワークフロー

`promote_short_term_to_mid_term` は次の順序で動作します。

1. 対象セッションの短期メッセージ一覧を取得
2. 空ならスキップ
3. 同じモデルで要約用プロンプトを投げ、3文要約を生成
4. `MemoryStore::promote_to_mid_term` で要約を保存
5. 保存成功後に短期記憶をクリア

トリガーは主に 2 つです。

- 短期記憶の圧縮閾値到達時
- `/clear` 実行時（セッション削除前）

## 長期記憶抽出ワークフロー

1. 応答文から JSON 配列を抽出するための専用プロンプトを送信
2. 応答を `Vec<ExtractedFact>` としてパース
3. パース失敗時は文字列中の `[`...`]` 部分を再試行
4. 空でなければ `MemoryStore::extract_long_term` で保存

保存データは `(fact, tags)` と `user_id`（存在する場合）です。

## セッション操作ワークフロー

- `get_history`: セッション取得後にクローンを返す
- `clear_session`:
  1. 可能なら中期記憶へ昇格
  2. 短期記憶を削除
  3. `SessionManager` から対象セッションを削除

## エラー時の挙動

- モデル呼び出しや保存で失敗した場合は `Result::Err` を返却
- 中期昇格/長期抽出は失敗しても本体応答は継続し、`warn` ログで通知
- `.config/INSTRUCTION.md` がない場合は初期化失敗

## 連携ポイント

- 入力: `nekoai-discord`（`ask` コマンド）
- 設定: `nekoai-config`（モデル/API/パラメータ）
- 記憶: `nekoai-memory`
- 型: `nekoai-domain::agent::session::SessionKey`
