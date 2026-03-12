#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 4 ]]; then
  echo "usage: $0 <batch_dir> <output_dir> [jobs] [snapshot_dir]" >&2
  exit 1
fi

BATCH_DIR="$1"
OUTPUT_DIR="$2"
JOBS="${3:-4}"
SNAPSHOT_DIR="${4:-${OUTPUT_DIR%/}/snapshots}"
UPDATE_SNAPSHOTS="${RPG_UPDATE_SNAPSHOTS:-0}"
JAVA_TARGET="${RPG_JAVA_TARGET:-java21}"
BATCH_OUT="${OUTPUT_DIR%/}/batch"
METRICS_CSV="${OUTPUT_DIR%/}/metrics.csv"
PERF_JSON="${OUTPUT_DIR%/}/metrics.summary.json"

mkdir -p "$OUTPUT_DIR"
if [[ ! -d "$BATCH_DIR" ]]; then
  echo "[rpg2java] error: batch_dir does not exist: $BATCH_DIR" >&2
  exit 2
fi

echo "[rpg2java] batch_dir=$BATCH_DIR output_dir=$OUTPUT_DIR jobs=$JOBS java_target=$JAVA_TARGET"
echo "[rpg2java] batch_out=$BATCH_OUT snapshot_dir=$SNAPSHOT_DIR update_snapshots=$UPDATE_SNAPSHOTS"

cmd=(
  cargo run --
  --batch-dir "$BATCH_DIR"
  --output-dir "$BATCH_OUT"
  --snapshot-dir "$SNAPSHOT_DIR"
  --mode auto
  --java-target "$JAVA_TARGET"
  --jobs "$JOBS"
  --metrics-csv "$METRICS_CSV"
  --perf-report-json "$PERF_JSON"
)

if [[ "$UPDATE_SNAPSHOTS" == "1" ]]; then
  cmd+=(--update-snapshots)
fi

"${cmd[@]}"

echo "[rpg2java] done"
echo "[rpg2java] java/report output: $BATCH_OUT"
echo "[rpg2java] snapshots: $SNAPSHOT_DIR"
echo "[rpg2java] metrics: $METRICS_CSV"
echo "[rpg2java] summary: $PERF_JSON"
