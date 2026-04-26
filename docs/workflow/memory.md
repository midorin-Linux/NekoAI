# `nekoai-memory` クレートのワークフロー

## 役割

`nekoai-memory` は短期・中期・長期の 3 層記憶を統合管理し、検索（recall）と保存（promotion/extraction）を提供します。外部のベクトル DB と埋め込みモデルの差異は内部で吸収します。

## 主な構成

- `store.rs`: 3 層統合インターフェース（`MemoryStore`）
- `short_term.rs`: セッション内インメモリ記憶（`DashMap`）
- `mid_term.rs`: 会話サマリー保存・検索・保持期間クリーンアップ
- `long_term.rs`: 重要事実保存・検索・削除
- `embedding.rs`: 埋め込み生成（OpenAI 互換 + Mock フォールバック）
- `vector_db/mod.rs`: ベクトル DB 抽象インターフェース
- `vector_db/qdrant.rs`: Qdrant 実装
- `vector_db/inmemory.rs`: インメモリ実装（テスト用途）

## 初期化ワークフロー（`MemoryStore::new` + `initialize`）

1. `&AppConfig` から設定を読み込み
   - `config.memory.short_term_max_entries`: 短期記憶上限
   - `config.memory.vector_db.*`: Qdrant URL、API key、コレクション名
   - `config.memory.mid_term_retention_days`: 中期記憶保持期間
   - `config.memory.mid_term_top_k` / `config.memory.long_term_top_k`: 検索結果数
   - `config.provider.embedding_model.*`: 埋め込みモデル設定
2. Qdrant クライアントを初期化（URL/API key）
3. 埋め込みモデルを初期化
   - 成功: `OpenAICompatibleEmbedder`
   - 失敗: `MockEmbedder` へフォールバック（`warn` ログ出力）
4. `MidTermMemory` / `LongTermMemory` を構築
5. `initialize().await` で両コレクションを `ensure_collection`

### テスト用コンストラクタ（`with_components`）

テストやモック環境で使用する別のコンストラクタです。既存の `mid_term`, `long_term`, `embedder` を直接注入できます。

## 短期記憶ワークフロー（`ShortTermMemory`）

1. `push_turn(session_key, user, assistant)` で 2 エントリ（User/Assistant）を追加
2. `max_entry` を超えた古いエントリは先頭から削除
3. `get_messages` で会話ログ取得
4. `clear` でセッション単位に削除
5. `get_count(session_key)` で現在のエントリ数を取得

短期記憶は完全にインメモリです。

### Role タイプ

短期記憶の各エントリは以下の役割を持ちます：
- `User`: ユーザーの入力
- `Assistant`: アシスタントの応答
- `Tool`: ツール呼び出しの結果

## 想起ワークフロー（`MemoryStore::recall`）

1. `recall(session_key, query)` を呼び出し
2. 内部で `embedder.embed(query)` を実行してクエリの埋め込みを生成
3. `mid_term.search_with_embedding(session_key, embedding, mid_term_top_k)` を実行
4. `long_term.search_with_embedding(session_key, embedding, long_term_top_k)` を実行
5. それぞれの結果を `RecalledMemory { mid_term, long_term }` で返却

検索は session スコープ（`guild_id`, `channel_id`, `kind`）でフィルタされます。

### `should_summarize` メソッド

短期記憶が `max_entry` に達したかどうかを判定し、中期記憶への昇格が必要かどうかを判断します。

## 中期記憶ワークフロー（`promote_to_mid_term`）

1. 対象セッションの短期メッセージ群を取得
2. 呼び出し元（`agent`）が生成した要約文を受け取る
3. 要約文を埋め込み化
4. `mid_term` コレクションへ upsert

### 保存される payload 構造

- `content`: 要約文
- `guild_id`: ギルド ID（DM の場合は `null`）
- `channel_id`: チャンネル ID
- `kind`: セッション種別（`guild`, `thread`, `dm`）
- `created_at`: 作成時刻（Unix タイムスタンプ）
- `message_count`: 元メッセージ数

保存後、短期記憶はクリアされます。

## 長期記憶ワークフロー（`extract_long_term`）

1. 呼び出し元から `facts: Vec<(String, Vec<String>)>` を受け取る（事実、タグ）
2. 各 fact を埋め込み化
3. `long_term` コレクションへ upsert
4. `user_id` があれば payload に格納

### 保存される payload 構造

- `content`: 事実（fact）
- `guild_id`: ギルド ID（DM の場合は `null`）
- `channel_id`: チャンネル ID
- `kind`: セッション種別（`guild`, `thread`, `dm`）
- `created_at`: 作成時刻（Unix タイムスタンプ）
- `tags`: タグ配列
- `user_id`: ユーザー ID（オプション）

## 長期記憶の検索・削除

### 検索方法

- `search(session_key, query, top_k)`: セッションスコープで検索
- `search_by_guild(guild_id, query, top_k)`: ギルド全体の事実を検索
- `search_by_user(user_id, query, top_k)`: ユーザー固有の事実を検索

### 削除方法

- `delete(id)`: ID で指定された事実を削除
- `delete_by_channel(channel_id)`: チャンネル単位で削除

## 保持期間クリーンアップワークフロー

`start_cleanup_job` はバックグラウンドタスクで日次実行します。

1. 24 時間ごとの interval を起動
2. `mid_term.delete_old_entries()` を実行
3. `created_at < cutoff` のデータを削除
4. 削除件数をログ出力

**注意**: このタスクは `tokio::spawn` で非同期タスクとして実行されます。

長期記憶は期限削除せず、明示削除のみです。

## ベクトル DB ワークフロー

`VectorDbClient` で以下操作を共通化しています。

- `upsert`: ベクトルとペイロードを保存
- `search`: ベクトル検索（類似度順にソート）
- `delete`: ID で指定されたポイントを削除
- `delete_by_filter`: フィルタ条件でポイントを削除
- `ensure_collection`: コレクションの作成・存在確認

### Qdrant 実装

- `QdrantClient` は一度生成されたクライアントを保持（リクエストごとに生成ではない）
- `SearchFilter` を Qdrant `Filter` に変換
- `must/should` 条件を適用して検索
- `session_scope_filter`: セッションスコープでフィルタリング（`guild_id`, `channel_id`, `kind`）
- `session_kind_value`: `SessionKind` を文字列に変換（`guild` / `thread` / `dm`）

### InMemory 実装

- テスト用途の簡易実装
- コサイン類似度でランキング
- `SearchFilter` の `Match/Range` をローカル評価

## 埋め込みワークフロー

### `OpenAICompatibleEmbedder`

- `embed(text)` で埋め込みを生成
- API 失敗時は `warn` ログを出し、内部の `MockEmbedder` にフォールバック
- `dimension()` で次元数を返却

### `MockEmbedder`

- 入力文字列から FNV-1a ハッシュで stable seed を作成
- 疑似乱数ベクトルを生成（次元整合のみ担保）
- テストや API 失敗時のフォールバックとして使用

## ヘルパー関数

### `search_result_to_entry`

`SearchResult` を `MemoryEntry` に変換するヘルパー関数。

- `content`: payload から取得
- `score`: 検索スコア
- `created_at`: Unix タイムスタンプから `DateTime<Utc>` に変換
- `metadata`: payload 全体を保持

## 連携ポイント

- `nekoai-agent`: `recall`, `promote_to_mid_term`, `extract_long_term`, `push_short_term`, `should_summarize`
- `nekoai-config`: 記憶設定と接続先
- `nekoai-domain`: `SessionKey` スコープ
