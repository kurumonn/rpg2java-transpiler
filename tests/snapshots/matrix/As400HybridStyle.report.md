# RPG to Java Migration Report

## Summary

- Java Target: java21
- Javac Check: skipped
- Statements: 6
- Symbols: 8
- Diagnostics: 0

## Operation Matrix

| Operation | Count | Level |
|---|---:|---|
| CHAIN | 1 | stub |
| ELSE | 1 | implemented |
| ENDIF | 1 | implemented |
| EVAL | 1 | implemented |
| EXSR | 1 | stub |
| IF | 1 | implemented |

## Symbols

| Symbol | Type | Assigned Lines | Referenced Lines |
|---|---|---|---|
| CNT | number | 5 | 5 |
| IN90 | bool | - | 2 |
| KEY | unknown | - | 1 |
| LOGERR | unknown | - | 3 |
| MASTER | unknown | - | 1 |
| file | unknown | - | 1 |
| key | unknown | - | 1 |
| subroutine | unknown | - | 3 |

## Unsupported / TODO Items

- CHAIN: key=KEY, file=MASTER
- EXSR: subroutine=LOGERR

## Subroutine Routes

- MAIN -> LOGERR (line 3)

## Diagnostics

- none
