# `nekoai-domain` クレートのワークフロー

## 役割

`nekoai-domain` はクレート間で共有するドメイン型を定義します。現在はエージェントセッション識別に関する最小限の型群を提供しています。

## 主な構成

- `agent/session.rs`
  - `SessionKind`
  - `SessionKey`

## 型定義ワークフロー

### `SessionKind`

Discord 上の会話コンテキストを次の 3 種類に正規化します。

- `GuildChannel`: サーバー内通常チャンネル
- `Thread`: スレッド
- `DirectMessage`: DM

### `SessionKey`

セッション識別子として次を束ねます。

- `guild_id: Option<GuildId>`
- `channel_id: ChannelId`
- `thread_id: Option<ChannelId>`
- `kind: SessionKind`

この型は `Eq + Hash` を持つため、セッションマップのキーとして利用できます。

## 利用ワークフロー

1. `nekoai-discord` が受信イベントから `SessionKey` を生成
2. `nekoai-agent` が `SessionKey` ごとにセッションを取得/更新
3. `nekoai-memory` が `SessionKey` を使って検索フィルタを構築

## 設計上の位置づけ

- 外部 I/O 依存ロジックは持たない
- ビジネス境界の共通言語を提供する
- 上位層（CLI/Discord）と下位層（Agent/Memory）の接着点になる
