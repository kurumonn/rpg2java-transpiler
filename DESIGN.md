# DEV-10 Design: RPG to Java Converter

## Goal

Build a migration tool that converts RPG assets to Java in a controlled and auditable way.

## Completion Criteria

1. Converter can parse target RPG subset with deterministic behavior.
2. Generated Java compiles under CI for supported input set.
3. Unsupported syntax is listed in migration report.
4. Regression suite includes real sample programs and snapshot outputs.
5. Security review completed (file handling, parser bounds, output sanitization).

## Architecture

1. `lexer`: tokenize RPG source (fixed/free format switch).
2. `parser`: build AST and keep source positions.
3. `analyzer`: symbol table, type guesses, call graph.
4. `ir`: normalized intermediate representation for codegen.
5. `java_codegen`: emit Java files from IR and templates.
6. `reporter`: conversion metrics and unsupported list.

## Phase Plan

### Phase P0 (current)

- CLI scaffold
- line-based parser
- Java skeleton output

### Phase P1

- fixed-format column parser
- operation catalog (`CHAIN`, `SETLL`, `READE`, `DOU`, `EXSR`, ...)
- syntax diagnostics

Status (2026-03-09):

- basic fixed-format `C` spec parsing: done
- operation matrix summary: done
- supported now: `EVAL`, `MOVEL`, `IF/ELSE/ENDIF`, `DOU/ENDDO`, `CALLP`, `READ/WRITE`
- stub mapping now: `CHAIN`, `SETLL`, `READE`, `EXSR`
- next P1 target: exact column grammar and indicator conditions

### Phase P2

- typed IR
- data structure mapping to Java classes
- subroutine to method conversion

Status (2026-03-09):

- IR transform layer added (`parser::Stmt` -> `ir::IrStmt`)
- symbol tracking added (assigned/referenced variables)
- basic type inference added (`number/text/bool/unknown`)
- Java emitter now consumes IR and emits typed local variable declarations

### Phase P3

- compile-ready Java output
- I/O adapter template
- unit tests + snapshot tests

Status (2026-03-09):

- IR-based emitter now generates typed local declarations
- `CALLP` procedure stubs are auto-generated
- condition fallback (`truthy`) added for non-boolean legacy condition input
- next P3 target: Java syntax-level verification tests and output snapshots

### Phase P4

- migration report HTML/JSON
- performance tuning for large source sets
- CLI batch mode and parallel conversion

Status (2026-03-09):

- JSON/Markdown migration report output implemented
- report includes operation matrix, TODO/unsupported items, diagnostics
- batch conversion mode implemented (`--batch-dir`)
- snapshot verification/update mode implemented (`--snapshot-dir`, `--update-snapshots`)
- parallel batch conversion implemented (`--jobs`)
- batch now continues on file-level failures and returns aggregated failure result
- batch metrics CSV export implemented (`--metrics-csv`): elapsed time / TODO rate / success-failure summary

## Ticket Drafts

- DEV-10-1: Parser foundation and operation matrix
- DEV-10-2: IR model and symbol tracking
- DEV-10-3: Java code generation templates
- DEV-10-4: Unsupported syntax reporting
- DEV-10-5: Batch mode + CI compile validation
