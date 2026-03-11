# Real Corpus Operations Guide

このドキュメントは、実案件RPG資産を安全に変換検証するための運用手順です。

## 1. 前提

- 実コーパスは **リポジトリ外** に配置する
- 機密データは匿名化済みであること
- 実行環境に Rust/Cargo があること

推奨ディレクトリ構成:

```text
/secure/rpg-corpus/
  ├─ source/      # 匿名化済み元データ（.rpg/.txt）
  ├─ snapshots/   # 期待出力スナップショット
  ├─ out/         # 変換結果
  └─ logs/        # 実行ログ
```

## 2. 実行コマンド

単発検証:

```bash
RPG_REAL_CORPUS_DIR=/secure/rpg-corpus/source \
RPG_REAL_CORPUS_JOBS=6 \
cargo test real_corpus_conversion_pipeline -- --nocapture
```

スナップショット更新（意図的変更時のみ）:

```bash
RPG_REAL_CORPUS_DIR=/secure/rpg-corpus/source \
RPG_REAL_CORPUS_JOBS=6 \
RPG_UPDATE_SNAPSHOTS=1 \
cargo test real_corpus_conversion_pipeline -- --nocapture
```

## 3. 運用スクリプト

`scripts/real_corpus_pipeline.sh` を使うと、バッチ変換 + メトリクス + 性能サマリを一括実行できます。

```bash
./scripts/real_corpus_pipeline.sh /secure/rpg-corpus/source /secure/rpg-corpus/out 6
```

出力:

- `out/batch/*.java`
- `out/batch/*.report.json`
- `out/batch/*.report.md`
- `out/metrics.csv`
- `out/metrics.summary.json`

## 4. 判定基準

- 失敗件数 `failed=0`
- `metrics.summary.json` の `recommendations` に重大事項がない
- TODO率が高すぎるファイルは別途移行計画を作成

## 5. セキュリティ注意

- 実データを `tests/fixtures` 配下へコピーしない
- CIへ機密コーパスをアップロードしない
- ログには個人情報/顧客情報を含めない
