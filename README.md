# rpg2java-transpiler

RPGソースコードをJavaへ段階的に移行するためのトランスパイラです。  
Rustで実装されており、現時点では「安全に移行を進めるための土台」を主目的にしています。

## 1. できること

- RPG風ソース（free/fixed）を解析してJavaスケルトンを生成
- 不明命令を黙って捨てず、`// TODO:` として残す
- 変換レポート（JSON/Markdown）を出力
- バッチ変換（ディレクトリ単位）とCSVメトリクス出力
- スナップショット比較による差分検証

## 2. 現在の対応範囲（Phase 0）

- `EVAL A = B` -> 代入
- `MOVEL X Y` -> 代入（`Y = X`）
- `IF / ELSE / ENDIF` -> Javaブロック
- `CALLP PROC` -> メソッド呼び出し
- `READ` / `WRITE` -> スタブAPI呼び出し
- 未対応命令 -> `// TODO:` コメント化
- 固定長フォーマット（`--mode fixed`）の基本解析
- 型付きIRとシンボルトラッキング（基本）
- Javaコンパイルしやすさ改善:
  - 推論型に基づくローカル変数宣言
  - `CALLP`先スタブメソッド自動生成
  - 条件式フォールバック `truthy(...)`

## 3. 前提環境

- Rust（推奨: stable）
- Cargo

インストール未済の場合:

```bash
curl https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
rustc --version
cargo --version
```

## 4. セットアップ

```bash
git clone https://github.com/kurumonn/rpg2java-transpiler.git
cd rpg2java-transpiler
cargo build
```

リリースビルド:

```bash
cargo build --release
```

## 5. 使い方（単一ファイル変換）

標準出力へ生成:

```bash
cargo run -- --input ./examples/sample.rpg --class-name LegacyOrderMain
```

ファイルへ書き出し:

```bash
cargo run -- \
  --input ./examples/sample.rpg \
  --output ./out/LegacyOrderMain.java \
  --class-name LegacyOrderMain
```

固定長フォーマットを明示:

```bash
cargo run -- \
  --input ./examples/sample_fixed.rpg \
  --mode fixed \
  --class-name LegacyFixedMain
```

`--mode` の指定値:

- `auto`（既定）
- `free`
- `fixed`

## 6. レポート出力

```bash
cargo run -- \
  --input ./examples/sample.rpg \
  --class-name LegacyOrderMain \
  --report-json ./out/report.json \
  --report-md ./out/report.md
```

## 7. バッチ変換（ディレクトリ単位）

```bash
cargo run -- \
  --batch-dir ./examples \
  --output-dir ./out/batch \
  --mode auto \
  --jobs 4 \
  --metrics-csv ./out/batch_metrics.csv
```

バッチモードの挙動:

- `.rpg` / `.txt` ファイルを対象に処理
- 一部失敗しても他ファイルの処理を継続
- 最後に `success/failed` を集計表示
- 1件でも失敗があれば終了コードは非0

## 8. スナップショット検証

初回作成/更新:

```bash
cargo run -- \
  --input ./examples/sample.rpg \
  --output ./out/LegacyOrderMain.java \
  --snapshot-dir ./out/snapshots \
  --update-snapshots
```

差分チェック:

```bash
cargo run -- \
  --input ./examples/sample.rpg \
  --output ./out/LegacyOrderMain.java \
  --snapshot-dir ./out/snapshots
```

## 9. テスト実行

互換マトリクステスト:

```bash
cargo test compat_matrix
```

対象フィクスチャ:

- RPG III fixed（算術/制御）
- RPG IV fixed（`SETLL` / `READE`）
- ILE RPG `/FREE`（ループ/プロシージャ呼び出し）
- AS400ハイブリッド（`CHAIN` / `EXSR`）

実コーパスE2E（ローカル限定）:

```bash
export RPG_REAL_CORPUS_DIR=/path/to/anonymized_or_internal_rpg_corpus
export RPG_REAL_CORPUS_JOBS=6
# export RPG_UPDATE_SNAPSHOTS=1  # 意図的に更新する時だけ
cargo test real_corpus_conversion_pipeline -- --nocapture
```

注意:

- `RPG_REAL_CORPUS_DIR` 未設定なら自動スキップ
- 機密ソースはリポジトリ外で管理

実コーパスの運用手順:

- `docs/CORPUS_OPERATIONS.md`

実行補助スクリプト:

- `scripts/real_corpus_pipeline.sh`（実コーパス一括変換）
- `scripts/perf_smoke.sh`（性能スモーク計測）

```bash
# 実コーパス一括変換
./scripts/real_corpus_pipeline.sh /secure/rpg-corpus/source ./out/real-corpus 6

# 性能スモーク計測
./scripts/perf_smoke.sh ./examples/batch 4
```

## 10. 変換カバレッジ実測（2026-03-12）

ローカルで実際にバッチ変換を実行し、RPG -> Java 変換の到達度を計測した結果です。

### 10.1 既存コーパス（8ファイル）

実行コマンド:

```bash
cargo run -- --batch-dir /tmp/rpg_cov_suite/input --output-dir /tmp/rpg_cov_suite/out --mode auto --jobs 4 --metrics-csv /tmp/rpg_cov_suite/metrics.csv --perf-report-json /tmp/rpg_cov_suite/perf.summary.json
```

結果:

- success: `8`
- failed: `0`
- files: `8`
- total_statements: `50`
- total_symbols: `30`
- op_implemented: `39`
- op_stub: `10`
- op_planned: `0`
- total_todos: `10`
- files_with_todo: `4`

TODO主要内訳:

- `EXSR`: 3
- `READE`: 3
- `SETLL`: 3
- `CHAIN`: 1

### 10.2 拡張コーパス（32ファイル, 網羅寄り）

24件の合成RPGケースを追加し、既存8件と合わせて検証しました。

`--mode auto`:

```bash
cargo run -- --batch-dir /tmp/rpg_cov_suite/input_all --output-dir /tmp/rpg_cov_suite/out_all --mode auto --jobs 4 --metrics-csv /tmp/rpg_cov_suite/metrics_all.csv --perf-report-json /tmp/rpg_cov_suite/perf_all.summary.json
```

- success: `32`
- failed: `0`
- total_statements: `145`
- implemented_ops: `96`
- stub_ops: `10`
- total_todos: `29`
- files_with_todo: `13`
- avg_todo_rate: `0.2365`

`--mode free`:

```bash
cargo run -- --batch-dir /tmp/rpg_cov_suite/input_all --output-dir /tmp/rpg_cov_suite/out_all_free --mode free --jobs 4 --metrics-csv /tmp/rpg_cov_suite/metrics_all_free.csv --perf-report-json /tmp/rpg_cov_suite/perf_all_free.summary.json
```

- success: `32`
- failed: `0`
- implemented_ops: `81`
- stub_ops: `0`
- total_todos: `44`
- avg_todo_rate: `0.2760`

観測ポイント:

- 変換処理としての成功率は `100%`（32/32）
- `auto` は `free` より TODO 率が低く、混在入力に対して有利
- 未実装の主因は `EXSR/READE/SETLL/CHAIN` 系

環境制約:

- 本計測環境では `javac` 未導入のため、Javaコンパイル実行結果は未計測です（`javac: command not found`）

## 11. CI

`.github/workflows/ci.yml` で以下を自動実行します。

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --tests`
- 生成Javaの `javac` コンパイル検証（Java 21）

## 12. CLI引数一覧

単体変換:

```text
--input <file>                  入力ファイル（-i）
--output <file>                 出力Javaファイル（-o）
--class-name <name>             生成クラス名（既定: MainProgram）
--java-target <name>            Java出力ターゲット（java21|java25-stable, 既定: java21）
--mode auto|free|fixed          解析モード（既定: auto）
--report-json <file>            JSONレポート出力
--report-md <file>              Markdownレポート出力
--snapshot-dir <dir>            スナップショット保存先
--update-snapshots              スナップショット更新
--javac-check                   生成Javaをコンパイル検証（singleは --output 必須）
--javac-cmd <cmd>               コンパイル検証コマンド（既定: javac）
--strict                         未対応命令（TODO化対象）をエラー扱い
```

バッチ変換:

```text
--batch-dir <dir>               入力ディレクトリ
--output-dir <dir>              出力ディレクトリ（既定: ./out/batch）
--java-target <name>            Java出力ターゲット（java21|java25-stable, 既定: java21）
--mode auto|free|fixed          解析モード
--jobs <n>                      並列数（1以上）
--snapshot-dir <dir>            スナップショット保存先
--update-snapshots              スナップショット更新
--metrics-csv <file>            CSVメトリクス出力（バッチ専用）
--perf-report-json <file>       改善提案付き性能サマリJSON出力（バッチ専用）
--javac-check                   各生成Javaをコンパイル検証
--javac-cmd <cmd>               コンパイル検証コマンド（既定: javac）
--strict                         未対応命令（TODO化対象）をエラー扱い
```

`--strict` を指定した場合、未対応命令（`// TODO` 化対象）が1件でも含まれると失敗終了します。

`--javac-check` を指定した場合、`report.json` / `report.md` に `javac_check` 結果（command/success/exit_code/detail）を出力します。  
失敗時もレポートを出力したうえでコマンド全体は失敗終了します。

`--metrics-csv` を指定した場合、同じディレクトリに `*.summary.json`（性能改善提案付きサマリ）を自動生成します。  
`--perf-report-json` を指定すると出力先を明示できます。

ヘルプ:

```bash
cargo run -- --help
```

## 13. 変換品質の考え方

- 未知構文は落とさず `TODO` 化して可視化
- 段階移行向けに「まず動く骨格」を優先
- 変換後の人手レビューを前提に設計

## 14. セーフティ設計

- 動的コード実行なし
- コンバータ内部でネットワークアクセスなし
- 入力はプレーンテキストとしてのみ扱う
- 未知構文の黙殺なし（必ず `TODO` として残す）

## 15. 開発ロードマップ

- Phase 1: fixed-format RPG lexer/parser（進行中）
- Phase 2: 型付きIRとシンボルテーブル（進行中）
- Phase 3: Javaテンプレート強化とコンパイル適合性改善（進行中）
- Phase 4: 意味検証・テスト拡張・差分レポート強化

## 16. よくあるエラー

- `either --input or --batch-dir is required`
  - `--input` か `--batch-dir` のどちらかを指定してください。
- `--input and --batch-dir are mutually exclusive`
  - 単体モードとバッチモードは同時指定できません。
- `--jobs must be >= 1`
  - `--jobs` は1以上を指定してください。

## 17. ライセンス

MIT License を採用しています。  
詳細は [LICENSE](LICENSE) を参照してください。

## 18. 次のアップデート（商用レベル化）

README上の次期アップデート計画として、商用導入を見据えたタスク分解を `docs/COMMERCIALIZATION_TASK_BREAKDOWN.md` に整理しました。

- 変換精度/未実装命令対応
- 品質ゲートとテスト戦略
- リリース運用とサポート体制
- セキュリティ/コンプライアンス

詳細: `docs/COMMERCIALIZATION_TASK_BREAKDOWN.md`

## 19. 商用導入チェックリスト（着手しやすい項目）

まずは低コストで進められる運用整備から実施してください。

- [ ] `CHANGELOG.md` の更新運用を開始する
- [ ] リリース前チェックを `docs/RELEASE_POLICY.md` に沿って実施する（`./scripts/release_precheck.sh`）
- [ ] CI必須ゲート（fmt / clippy / tests / javac-check）の結果を記録する
- [ ] 実コーパス実行時の手順を `docs/CORPUS_OPERATIONS.md` で標準化する

関連ドキュメント:

- `docs/COMMERCIALIZATION_TASK_BREAKDOWN.md`
- `docs/RELEASE_POLICY.md`
- `CHANGELOG.md`

