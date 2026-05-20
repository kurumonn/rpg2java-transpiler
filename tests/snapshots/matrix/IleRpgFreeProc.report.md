# RPG to Java Migration Report

## Summary

- Java Target: java21
- Javac Check: skipped
- Statements: 5
- Symbols: 2
- Diagnostics: 0

## Operation Matrix

| Operation | Count | Level |
|---|---:|---|
| CALLP | 1 | implemented |
| DOU | 1 | implemented |
| ENDDO | 1 | implemented |
| EVAL | 2 | implemented |

## Top Unsupported Operations

- none

## Symbols

| Symbol | Type | Assigned Lines | Referenced Lines |
|---|---|---|---|
| 3 | number | - | - |
| IDX | number | 2,5 | 3,5 |

## Unsupported / TODO Items

- none

## Subroutine Routes

- none

## Source Map

| RPG Line | Java Line | Kind | Note |
|---:|---:|---|---|
| 2 | 7 | assign | IDX |
| 3 | 8 | loop | IDX *GT 3 |
| 4 | 9 | call | PROCESS_ROW |
| 5 | 10 | assign | IDX |
| 6 | 11 | enddo |  |

## Diagnostics

- none
