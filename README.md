# rpg2java-transpiler

RPG source code to Java conversion tool (Rust implementation, early stage).

## Scope

- Input: legacy RPG-like line statements
- Output: Java skeleton code
- Current goal: safe migration design and phased implementation baseline

## Current Support (Phase 0)

- `EVAL A = B` -> assignment
- `MOVEL X Y` -> assignment (`Y = X`)
- `IF / ELSE / ENDIF` -> Java block
- `CALLP PROC` -> method call
- `READ` / `WRITE` -> stub API call
- unknown statements -> `// TODO:` comment
- fixed-format mode (`--mode fixed`) basic support
- operation matrix summary output (implemented/stub/planned)
- typed IR generation and symbol tracking (basic)
- compile-friendly Java output improvements:
  - generated local variable declarations from inferred types
  - `CALLP` target stub methods auto-generated
  - condition fallback via `truthy(...)`

## Usage

```bash
cargo run -- --input ./examples/sample.rpg --class-name LegacyOrderMain
```

or write to file:

```bash
cargo run -- --input ./examples/sample.rpg --output ./out/LegacyOrderMain.java --class-name LegacyOrderMain
```

fixed-format parse:

```bash
cargo run -- --input ./examples/sample_fixed.rpg --mode fixed --class-name LegacyFixedMain
```

parse mode:

- `--mode auto` (default)
- `--mode free`
- `--mode fixed`

report output:

- `--report-json ./out/report.json`
- `--report-md ./out/report.md`

batch mode:

```bash
cargo run -- --batch-dir ./examples --output-dir ./out/batch --mode auto --jobs 4 --metrics-csv ./out/batch_metrics.csv
```

batch behavior:

- continues processing even if some files fail
- prints final summary (`success/failed`)
- exits non-zero if any file failed
- optional CSV metrics (`--metrics-csv`) with per-file elapsed time and TODO rate

snapshot verify:

```bash
cargo run -- --input ./examples/sample.rpg --output ./out/LegacyOrderMain.java --snapshot-dir ./out/snapshots --update-snapshots
cargo run -- --input ./examples/sample.rpg --output ./out/LegacyOrderMain.java --snapshot-dir ./out/snapshots
```

version compatibility test:

```bash
cargo test compat_matrix
```

covered fixture profiles:

- RPG III fixed-style arithmetic/control
- RPG IV fixed-style file I/O (`SETLL`/`READE`)
- ILE RPG `/FREE` style loop/procedure call
- AS400 hybrid style with legacy ops (`CHAIN`/`EXSR`)

real corpus compatibility test (local only):

```bash
export RPG_REAL_CORPUS_DIR=/path/to/anonymized_or_internal_rpg_corpus
export RPG_REAL_CORPUS_JOBS=6
# export RPG_UPDATE_SNAPSHOTS=1   # only when intentionally updating snapshots
cargo test real_corpus_conversion_pipeline -- --nocapture
```

notes:

- test is skipped automatically when `RPG_REAL_CORPUS_DIR` is not set
- keep confidential source outside repository

## Safety Design

- No dynamic code execution
- No network access in converter
- Input is treated as plain text only
- Unknown syntax is never silently dropped (kept as `TODO` comment)

## Next Phases

- Phase 1: fixed-format RPG lexer/parser (in progress)
- Phase 2: typed intermediate representation and symbol table (in progress)
- Phase 3: Java domain templates and compile-ready output (in progress)
- Phase 4: semantic verification, test suite, diff report
