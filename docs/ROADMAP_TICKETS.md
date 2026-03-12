# Roadmap Tickets (Execution Backlog)

最終更新: 2026-03-12

このファイルは、`README` と `DESIGN.md` のロードマップを実装可能な単位へ分解した実行台帳です。  
ステータスは `todo` / `in_progress` / `done` で管理します。

## Milestone M1: Phase 1 パーサ安定化

| Ticket | Status | Scope | 受け入れ基準 |
|---|---|---|---|
| RPG2J-101 | done | fixed-formatの厳密列定義（C仕様中心） | 列ズレ入力に対し診断が行番号付きで出る |
| RPG2J-102 | done | 指標条件（indicator）の構文解析強化 | IF条件にindicatorを含むケースがAST/IRへ反映される |
| RPG2J-103 | done | 命令カタログ拡充（`CHAIN/SETLL/READE/EXSR/DOU`） | `TODO` 出力が命令別に安定し、誤検出率を低減 |
| RPG2J-104 | done | エラー診断標準化（severity/code） | JSON/MDレポートに同一形式で出力される |

## Milestone M2: Phase 2 IR/型解析強化

| Ticket | Status | Scope | 受け入れ基準 |
|---|---|---|---|
| RPG2J-201 | done | データ定義（数値/文字/論理）の推論精度改善 | 型誤推論ケースの回帰テストを追加し通過 |
| RPG2J-202 | done | 式評価IRの拡張（比較/算術/論理） | 代表式のJava出力が型整合する |
| RPG2J-203 | done | サブルーチン解析（`EXSR` 経路） | 呼出経路がレポートで追跡可能 |
| RPG2J-204 | done | シンボル表に参照/代入位置を保持 | 変数ごとの使用箇所がレポート出力される |

## Milestone M3: Phase 3 変換品質強化

| Ticket | Status | Scope | 受け入れ基準 |
|---|---|---|---|
| RPG2J-301 | done | Java生成テンプレート整備（可読性） | 代表サンプルで命名衝突/未宣言が解消 |
| RPG2J-302 | done | `javac` 通過率改善（未対応構文フォールバック） | 実コーパスで `javac-check` 成功率の改善を確認 |
| RPG2J-303 | done | `java21/java25-stable` 差分方針を明文化 | 出力差分ポリシーをREADMEへ反映 |
| RPG2J-304 | done | スナップショットテスト拡張 | fixed/free/hybrid別の回帰セットを追加 |

## Milestone M4: Phase 4 実運用検証

| Ticket | Status | Scope | 受け入れ基準 |
|---|---|---|---|
| RPG2J-401 | done | 実コーパス回帰パイプライン運用定義 | `docs/CORPUS_OPERATIONS.md` と手順が一致 |
| RPG2J-402 | done | 失敗分析レポート（原因分類） | 失敗理由をカテゴリ集計できる |
| RPG2J-403 | done | 性能レポート改善（閾値/推奨精度） | `perf.summary.json` の推奨が実測に即した内容 |
| RPG2J-404 | done | セキュリティ/入力境界テストの定期化 | 異常入力テストがCIで継続実行される |

## Cross-Cutting

| Ticket | Status | Scope | 受け入れ基準 |
|---|---|---|---|
| RPG2J-900 | done | CIの`javac`互換検証をJava 21/25のマトリクス化 | `.github/workflows/ci.yml` で21/25両方が実行される |
| RPG2J-901 | done | READMEの運用明確化（対象範囲/非対象範囲） | ユーザーが「完全自動変換」の誤解をしない |

## 実行順（推奨）

1. M1 (`RPG2J-101`〜`104`)
2. M2 (`RPG2J-201`〜`204`)
3. M3 (`RPG2J-301`〜`304`)
4. M4 (`RPG2J-401`〜`404`)

## 今回着手分

- 完了: `RPG2J-900`（CI Java 21/25 matrix）
- 完了: `RPG2J-901`（README明確化）
- 完了: `RPG2J-101`（fixed列ズレの行/列診断 + E2Eテスト追加）
- 完了: `RPG2J-102`（IF/DOUのindicator条件正規化 + Java出力反映）
- 完了: `RPG2J-103`（stub命令の引数正規化 + free/fixedカタログ整備）
- 完了: `RPG2J-104`（diagnosticsのseverity/code/line/column標準化）
- 完了: `RPG2J-201`（型推論: 代入伝播 / 文字列比較 / indicator・*ON/*OFF対応）
- 完了: `RPG2J-202`（比較/論理演算子の式正規化 + Java文字列化 + cast最適化）
- 完了: `RPG2J-203`（EXSRサブルーチン経路をJSON/Markdownへ出力）
- 完了: `RPG2J-204`（シンボル別の代入/参照行をIR・レポートへ出力）
- 完了: `RPG2J-301`（Java命名衝突回避: class名/args/ヘルパー/CALLPの安全リネーム）
- 完了: `RPG2J-302`（未対応式/条件のJavaフォールバック + `*CAT` 文字列連結変換）
- 完了: `RPG2J-303`（`java21/java25-stable` 差分ポリシー明文化 + 回帰テスト）
- 完了: `RPG2J-304`（fixed/free/hybrid のJava/JSON/MDスナップショット回帰追加）
- 完了: `RPG2J-401`（実コーパス運用手順をスクリプト仕様へ統一 + E2E検証）
- 完了: `RPG2J-402`（perf.summary.json に失敗カテゴリ集計とサンプルを追加）
- 完了: `RPG2J-403`（perf.summary.json に thresholds/signals を追加し推奨の根拠を明示）
- 完了: `RPG2J-404`（不正UTF-8/危険な識別子/長大入力の境界テストをCI対象化）
