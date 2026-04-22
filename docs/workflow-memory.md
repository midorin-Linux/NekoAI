# `nekoai-memory` クレートのワークフロー

## 役割

`nekoai-memory` は短期・中期・長期の 3 層記憶を統合管理し、検索（recall）と保存（promotion/extraction）を提供します。外部のベクトル DB と埋め込みモデルの差異は内部で吸収します。

## 主な構成

- `store.rs`: 3 層統合インターフェース（`MemoryStore`）
- `short_term.rs`: セッション内インメモリ記憶（`DashMap`）
- `mid_term.rs`: 会話サマリー保存・検索・保持期間クリーンアップ
- `long_term.rs`: 重要事実保存・検索・削除
- `embedding.rs`: 埋め込み生成（OpenAI 互換 + Mock フォールバック）
- `vector_db/*`: ベクトル DB 抽象と Qdrant/インメモリ実装

## 初期化ワークフロー（`MemoryStore::new` + `initialize`）

1. `Config.memory` から短期記憶上限や top-k を読み込み
2. Qdrant クライアントを設定（URL/API key）
3. 埋め込みモデルを初期化
   - 成功: `OpenAICompatibleEmbedder`
   - 失敗: `MockEmbedder` へフォールバック
4. `MidTermMemory` / `LongTermMemory` を構築
5. `initialize().await` で両コレクションを `ensure_collection`

## 短期記憶ワークフロー（`ShortTermMemory`）

1. `push_turn(session_key, user, assistant)` で 2 エントリ（User/Assistant）を追加
2. `max_entry` を超えた古いエントリは先頭から削除
3. `get_messages` で会話ログ取得
4. `clear` でセッション単位に削除

短期記憶は完全にインメモリです。

## 想起ワークフロー（`MemoryStore::recall`）

1. `mid_term.search(session_key, query, mid_term_top_k)`
2. `long_term.search(session_key, query, long_term_top_k)`
3. それぞれの結果を `RecalledMemory { mid_term, long_term }` で返却

検索は session スコープ（`guild_id`, `channel_id`, `kind`）でフィルタされます。

## 中期記憶ワークフロー（`promote_to_mid_term`）

1. 対象セッションの短期メッセージ群を取得
2. 呼び出し元（`agent`）が生成した要約文を受け取る
3. 要約文を埋め込み化
4. `mid_term` コレクションへ upsert
5. 保存後、短期記憶をクリア

## 長期記憶ワークフロー（`extract_long_term`）

1. 呼び出し元から `facts: Vec<(fact, tags)>` を受け取る
2. 各 fact を埋め込み化
3. `long_term` コレクションへ upsert
4. `user_id` があれば payload に格納

## 保持期間クリーンアップワークフロー

`start_cleanup_job` はバックグラウンドタスクで日次実行します。

1. 24 時間ごとの interval を起動
2. `mid_term.delete_old_entries()` を実行
3. `created_at < cutoff` のデータを削除
4. 削除件数をログ出力

長期記憶は期限削除せず、明示削除のみです。

## ベクトル DB ワークフロー

`VectorDbClient` で以下操作を共通化しています。

- `upsert`
- `search`
- `delete`
- `delete_by_filter`
- `ensure_collection`

### Qdrant 実装

- リクエストごとにクライアントを生成
- `SearchFilter` を Qdrant `Filter` に変換
- `must/should` 条件を適用して検索

### InMemory 実装

- テスト用途の簡易実装
- コサイン類似度でランキング
- `SearchFilter` の `Match/Range` をローカル評価

## 埋め込みワークフロー

- `OpenAICompatibleEmbedder::embed`
  1. API で埋め込み生成
  2. 失敗時は warning を出し `MockEmbedder` にフォールバック
- `MockEmbedder`
  - 入力文字列から stable seed を作成
  - 疑似乱数ベクトルを生成（次元整合のみ担保）

## 連携ポイント

- `nekoai-agent`: `recall`, `promote_to_mid_term`, `extract_long_term`
- `nekoai-config`: 記憶設定と接続先
- `nekoai-domain`: `SessionKey` スコープ
