#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

run() {
  local label="$1"
  shift
  echo "[precheck] $label"
  "$@"
}

run "cargo fmt --check" cargo fmt --check
run "cargo clippy --all-targets --all-features -- -D warnings" cargo clippy --all-targets --all-features -- -D warnings
run "cargo test --tests" cargo test --tests

if command -v javac >/dev/null 2>&1; then
  mkdir -p out/precheck
  run "cargo run -- --input ./examples/sample.rpg --output ./out/precheck/LegacyOrderMain.java --class-name LegacyOrderMain --java-target java21 --javac-check --javac-cmd javac" \
    cargo run -- \
      --input ./examples/sample.rpg \
      --output ./out/precheck/LegacyOrderMain.java \
      --class-name LegacyOrderMain \
      --java-target java21 \
      --javac-check \
      --javac-cmd javac
else
  echo "[precheck] WARN: javac not found; skip javac-check."
fi

echo "[precheck] done"
