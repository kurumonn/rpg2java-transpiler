#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
  echo "usage: $0 <batch_dir> <output_dir> [jobs]" >&2
  exit 1
fi

BATCH_DIR="$1"
OUTPUT_DIR="$2"
JOBS="${3:-4}"
METRICS_CSV="${OUTPUT_DIR%/}/metrics.csv"

mkdir -p "$OUTPUT_DIR"

echo "[rpg2java] batch_dir=$BATCH_DIR output_dir=$OUTPUT_DIR jobs=$JOBS"

cargo run -- \
  --batch-dir "$BATCH_DIR" \
  --output-dir "$OUTPUT_DIR" \
  --mode auto \
  --jobs "$JOBS" \
  --metrics-csv "$METRICS_CSV"

echo "[rpg2java] done metrics=$METRICS_CSV summary=${METRICS_CSV%.csv}.summary.json"
