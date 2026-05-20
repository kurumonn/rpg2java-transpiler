# RPG to Java Migration Report

## Summary

- Java Target: java21
- Javac Check: skipped
- Statements: 4
- Symbols: 5
- Diagnostics: 0

## Operation Matrix

| Operation | Count | Level |
|---|---:|---|
| ENDIF | 1 | implemented |
| EVAL | 1 | implemented |
| IF | 1 | implemented |
| WRITE | 1 | implemented |

## Top Unsupported Operations

- none

## Symbols

| Symbol | Type | Assigned Lines | Referenced Lines |
|---|---|---|---|
| LIMIT | unknown | - | 2 |
| ORDER_REC | unknown | - | 3 |
| PRICE | number | - | 1 |
| TAX | number | - | 1 |
| TOTAL | number | 1 | - |

## Unsupported / TODO Items

- none

## Subroutine Routes

- none

## Source Map

| RPG Line | Java Line | Kind | Note |
|---:|---:|---|---|
| 1 | 10 | assign | TOTAL |
| 2 | 11 | if | LIMIT |
| 3 | 12 | write | ORDER_REC |
| 4 | 13 | endif |  |

## Diagnostics

- none
