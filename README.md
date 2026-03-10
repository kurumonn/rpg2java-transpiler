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

## 10. CLI引数一覧

単体変換:

```text
--input <file>                  入力ファイル（-i）
--output <file>                 出力Javaファイル（-o）
--class-name <name>             生成クラス名（既定: MainProgram）
--mode auto|free|fixed          解析モード（既定: auto）
--report-json <file>            JSONレポート出力
--report-md <file>              Markdownレポート出力
--snapshot-dir <dir>            スナップショット保存先
--update-snapshots              スナップショット更新
```

バッチ変換:

```text
--batch-dir <dir>               入力ディレクトリ
--output-dir <dir>              出力ディレクトリ（既定: ./out/batch）
--mode auto|free|fixed          解析モード
--jobs <n>                      並列数（1以上）
--snapshot-dir <dir>            スナップショット保存先
--update-snapshots              スナップショット更新
--metrics-csv <file>            CSVメトリクス出力（バッチ専用）
```

ヘルプ:

```bash
cargo run -- --help
```

## 11. 変換品質の考え方

- 未知構文は落とさず `TODO` 化して可視化
- 段階移行向けに「まず動く骨格」を優先
- 変換後の人手レビューを前提に設計

## 12. セーフティ設計

- 動的コード実行なし
- コンバータ内部でネットワークアクセスなし
- 入力はプレーンテキストとしてのみ扱う
- 未知構文の黙殺なし（必ず `TODO` として残す）

## 13. 開発ロードマップ

- Phase 1: fixed-format RPG lexer/parser（進行中）
- Phase 2: 型付きIRとシンボルテーブル（進行中）
- Phase 3: Javaテンプレート強化とコンパイル適合性改善（進行中）
- Phase 4: 意味検証・テスト拡張・差分レポート強化

## 14. よくあるエラー

- `either --input or --batch-dir is required`
  - `--input` か `--batch-dir` のどちらかを指定してください。
- `--input and --batch-dir are mutually exclusive`
  - 単体モードとバッチモードは同時指定できません。
- `--jobs must be >= 1`
  - `--jobs` は1以上を指定してください。

## 15. ライセンス

MIT License を採用しています。  
詳細は [LICENSE](LICENSE) を参照してください。
