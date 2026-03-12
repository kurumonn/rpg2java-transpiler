# Coverage Report (2026-03-12)

このドキュメントは、RPG -> Java 変換の実測結果を記録した詳細レポートです。  
公開READMEからは分離し、必要な人だけ参照する構成にしています。

## 1. 既存コーパス（8ファイル）

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

## 2. 拡張コーパス（32ファイル）

24件の合成RPGケースを追加し、既存8件と合わせて検証。

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

## 3. Javaターゲット方針

- `java25-stable` は出力ターゲット/プロファイル指定
- 変換本体は `java21` と同等（差分はメタ情報中心）
- 方針回帰テスト: `tests/java_target_policy_e2e.rs`

## 4. 環境制約

- 本計測環境では `javac` 未導入のため、Javaコンパイル実行結果は未計測の場合がある
