#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 3 ]]; then
  echo "usage: $0 <batch_dir> [jobs] [output_base_dir]" >&2
  exit 1
fi

BATCH_DIR="$1"
JOBS="${2:-4}"
BASE_DIR="${3:-./out/perf}"
STAMP="$(date +%Y%m%d-%H%M%S)"
OUT_DIR="${BASE_DIR%/}/${STAMP}"
METRICS_CSV="${OUT_DIR}/metrics.csv"
PERF_JSON="${OUT_DIR}/perf.summary.json"

mkdir -p "$OUT_DIR"

echo "[perf] batch_dir=$BATCH_DIR jobs=$JOBS out=$OUT_DIR"

cargo run -- \
  --batch-dir "$BATCH_DIR" \
  --output-dir "${OUT_DIR}/batch" \
  --jobs "$JOBS" \
  --metrics-csv "$METRICS_CSV" \
  --perf-report-json "$PERF_JSON"

echo "[perf] metrics=$METRICS_CSV"
echo "[perf] summary=$PERF_JSON"
