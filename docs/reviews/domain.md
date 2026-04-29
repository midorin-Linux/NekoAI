# nekoai-domain レビュー

## 対象
- `crates/domain/src/agent/session.rs`

## 指摘事項

### 1. [Medium] ドメイン層が `serenity` 型に直接依存している
- 該当: `crates/domain/src/agent/session.rs:1`
- `SessionKey` が `ChannelId` / `GuildId` を直接持つため、ドメインが Discord SDK に強く結合しています。
- 別チャットプラットフォーム対応やユニットテスト時の独立性が下がります。
- 修正案:
  - ドメイン層では `u64` / newtype（`GuildIdValue`, `ChannelIdValue`）を使う
  - Discord クレート側で SDK 型との変換を担当する

### 2. [Low] `SessionKey` の同一性定義が暗黙的で意図が読み取りづらい
- 該当: `crates/domain/src/agent/session.rs:11`
- `Eq/Hash` 由来で比較されますが、「thread では `thread_id` 優先」などの仕様がコードから明示されません。
- 修正案:
  - `SessionKey::for_guild_channel` / `SessionKey::for_thread` / `SessionKey::for_dm` のコンストラクタを用意して不変条件を固定する

### 3. [Low] `rig-core` が `Cargo.toml` に含まれているがドメイン層で未使用
- 該当: `crates/domain/Cargo.toml:8`
- ドメイン層のソースコードで `rig-core` を直接 import しておらず、不要な依存になっています。
- 修正案:
  - 使っていないなら依存から削除する

### 4. [Low] スレッドセッションで `channel_id` と `thread_id` が同一値になる冗長性
- 該当: `crates/domain/src/agent/session.rs:11`, `crates/discord/src/commands/utils/session_resolver.rs:13`
- スレッドチャンネルの場合、`channel_id`（Discord から得られるチャンネルID）と `thread_id`（`guild_channel.id`）が同一のスレッドIDになります。
- 修正案:
  - スレッド時は `channel_id` に親チャンネルを格納し、`thread_id` をスレッド専用にする設計も検討する
  - もしくはドキュメントでこの挙動を明記する
