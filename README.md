# rpg2java-transpiler

RPGソースをJavaへ段階移行するためのRust製トランスパイラです。  
目的は「完全自動変換」ではなく、変換可能部分を安全に自動化し、未対応箇所を `TODO` として可視化することです。

## 1. できること

- RPG風ソース（free/fixed）を解析してJavaスケルトンを生成
- 不明命令を黙って捨てず、`// TODO:` として残す
- 変換レポート（JSON/Markdown）を出力
- Javaターゲット切替（`java21` / `java25-stable`）
- バッチ変換（ディレクトリ単位）
- スナップショット比較による差分検証

## 2. 対応範囲（現状）

- `EVAL` / `MOVEL` / `IF` / `ELSE` / `ENDIF` / `DOU` / `ENDDO`
- `CALLP`（スタブメソッド生成）
- `READ` / `WRITE`（I/Oスタブ呼び出し）
- 固定長フォーマット（`--mode fixed`）の基本解析
- 型付きIRとシンボルトラッキング（基本）

未対応構文は `TODO` 化して、変換結果とレポートに残します。

## 3. セットアップ

```bash
git clone https://github.com/kurumonn/rpg2java-transpiler.git
cd rpg2java-transpiler
cargo build
```

## 4. 使い方

単一ファイル:

```bash
cargo run -- \
  --input ./examples/sample.rpg \
  --output ./out/Sample.java \
  --class-name Sample \
  --mode auto
```

レポート出力:

```bash
cargo run -- \
  --input ./examples/sample.rpg \
  --output ./out/Sample.java \
  --report-json ./out/Sample.report.json \
  --report-md ./out/Sample.report.md
```

バッチ変換:

```bash
cargo run -- \
  --batch-dir ./examples \
  --output-dir ./out/batch \
  --jobs 4
```

## 5. 主なCLIオプション

- `--input`, `--output`, `--class-name`
- `--batch-dir`, `--output-dir`, `--jobs`
- `--mode auto|free|fixed`
- `--java-target java21|java25-stable`
- `--report-json`, `--report-md`
- `--snapshot-dir`, `--update-snapshots`
- `--metrics-csv`, `--perf-report-json`
- `--javac-check`, `--javac-cmd`

```bash
cargo run -- --help
```

## 6. テスト

```bash
cargo test --tests
```

代表的な回帰テスト:

- `cargo test compat_matrix`
- `cargo test snapshot_matrix_e2e`
- `cargo test security_boundary_e2e`

## 7. 公開ポリシー

- 実コーパス（実案件データ）はリポジトリ外で管理
- 公開READMEには実運用導線の詳細は掲載しない
- 変換方針・CLI・検証方法を中心に公開

詳細方針:

- `docs/PUBLIC_RELEASE_POLICY.md`

## 8. 追加ドキュメント

- 変換カバレッジ実測: `docs/coverage-report.md`
- 開発台帳: `docs/ROADMAP_TICKETS.md`
- 実コーパス運用方針（公開版要約）: `docs/CORPUS_OPERATIONS.md`

## 9. ライセンス

MIT License
