# nekoai-tools レビュー

## 対象
- `crates/tools/src/lib.rs`

## 指摘事項

### 1. [Medium] クレートが実質空で、依存クレートとしての契約が未定義
- 該当: `crates/tools/src/lib.rs:1`
- 現状は公開 API がなく、他クレートから参照される前提が作れていません。
- 修正案:
  - 最低限 `Tool`, `ToolRegistry`, `ToolResult` などの骨組み trait/struct を定義する
  - 空実装でも「今後の拡張点」を型として固定しておく

### 2. [Low] 回帰検出のための最小テストも未整備
- 該当: `crates/tools/src/lib.rs:1`
- 現在テストが 0 件で、今後実装追加時にインターフェース崩壊を検知しづらいです。
- 修正案:
  - ダミーツール登録・実行の最小ユニットテストを先に置く

### 3. [Medium] どのクレートも `nekoai-tools` に依存しておらず、孤立したクレートになっている
- 該当: `crates/tools/Cargo.toml`, 全クレートの `Cargo.toml`
- `nekoai-tools` は workspace メンバーとして登録されていますが、`cli` / `agent` / `discord` のいずれのクレートも依存として参照していません。実質的に未使用です。
- 修正案:
  - `agent` クレートなどから参照されるように依存関係を設計に応じて追加する
  - 当面使わないなら workspace メンバーから一時的に除外する
