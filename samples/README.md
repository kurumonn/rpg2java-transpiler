# Sample Corpus

This directory contains the commercial PoC sample set referenced by the task.

Files:

- `free_basic.rpg` - free-format control flow, assignment, CALLP, READ, WRITE
- `free_with_stub_ops.rpg` - free-format stub ops: CHAIN, SETLL, READE, EXSR
- `fixed_basic.rpg` - fixed-format basic control flow and I/O
- `fixed_stub_ops.rpg` - fixed-format stub ops and subroutine routing
- `hybrid_mixed.rpg` - mixed free/fixed input for `--mode auto`
- `invalid_fixed_alignment.rpg` - diagnostic sample for fixed-column errors
- `business_order_flow.rpg` - larger business flow with numeric, text, and boolean inference

Recommended smoke path:

1. Convert `free_basic.rpg`.
2. Convert `business_order_flow.rpg`.
3. Compile the generated Java with `javac`.
4. Run the generated main class with `java`.
