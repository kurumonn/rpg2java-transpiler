# RPG2Java Commercial PoC Plan

This document defines the commercial PoC baseline for `v0.1.0-alpha`.

## Scope

- CLI conversion for single file and batch projects.
- JSON, Markdown, HTML, CSV, and source-map report artifacts.
- Java 21 and Java 25 stable target profiles.
- `javac` verification when a JDK is available.
- Docker-based offline execution.
- Safe handling of unsupported RPG operations through TODO diagnostics.

## Release Readiness Checklist

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --tests`
- [ ] Java 21 generated output compile check
- [ ] Java 25 stable generated output compile check
- [ ] Docker image builds
- [x] Sample corpus batch conversion succeeds
- [x] HTML report generated for sample corpus
- [x] Source map generated for sample corpus
- [ ] Release notes include limitations and unsupported operation policy
- [ ] Tag `v0.1.0-alpha`

## Release Automation

Tag pushes matching `v*` run `.github/workflows/release.yml`.

The release workflow:

- runs `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --tests`.
- builds Linux and Windows release binaries.
- packages README, DESIGN, docs, examples, and the binary.
- builds the Docker image and runs `rpg2java doctor` inside the container.
- publishes a draft prerelease through GitHub Releases.

Recommended first tag:

```bash
git tag v0.1.0-alpha
git push origin v0.1.0-alpha
```

## Commercial CLI Contract

Primary commands:

```bash
rpg2java convert --input sample.rpg --output out/Sample.java
rpg2java batch --batch-dir src-rpg --output-dir out --html-report --source-map-report
rpg2java doctor
```

Compatibility mode:

```bash
rpg2java --input sample.rpg --output out/Sample.java
```

Commercial flags:

- `--report-html <file>`: writes a customer-facing HTML report in single-file mode.
- `--source-map <file>`: writes RPG line to Java line mappings in single-file mode.
- `--html-report`: writes per-file HTML reports in batch mode.
- `--source-map-report`: writes per-file source maps in batch mode.
- `--strict`: fails when any TODO is emitted.
- `--fail-on-todo-rate <0.0-1.0>`: fails when TODO density exceeds the threshold.
- `--dry-run`: validates and reports planned work without writing output files.
- `--redact`: reserved for report masking.
- `--config`, `--profile`, `--baseline`, `--sarif`: reserved extension points.

## Source Map Contract

Schema version: `rpg2java-source-map.v1`.

Each mapping row contains:

- `rpg_line`
- `java_line`
- `kind`
- `note`

The source map is designed to support:

- RPG line to Java line navigation.
- Java TODO to RPG source navigation.
- side-by-side review UI.
- audit evidence for migrated business logic.

## Security Baseline

- RPG source is parsed as text and never executed.
- Docker usage is offline-first and volume based.
- Unknown or risky RPG behavior is emitted as TODO instead of being silently dropped.
- Reports should not include customer secrets once `--redact` is implemented.
- Real customer corpus must stay outside the public repository.

## Current Local Verification Note

Executed locally on Windows with `C:\Users\kurumonn\.cargo\bin\cargo.exe`:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --tests`
- `cargo test --test sample_corpus_e2e`

All passed.

Sample corpus verification covered:

- free format conversion and execution
- fixed format conversion and execution
- mixed free/fixed diagnostics
- batch mode report generation
- `javac` checking in batch mode

Docker CLI is installed, but Docker Desktop/Linux engine was not running locally:

```text
open //./pipe/dockerDesktopLinuxEngine: The system cannot find the file specified
```

Docker image verification is therefore delegated to the release workflow until the local engine is started.
