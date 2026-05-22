# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RISC-V simulator (RV32GC + Sv32 MMU) in Rust, targeting Linux boot. Currently implements RV32I base integer ISA.

## Build & Test Commands

```bash
cargo build                  # Build all crates
cargo test                   # Run unit tests (core + mem)
cargo clippy                 # Lint (ignore manual_is_multiple_of warnings)
cargo run -- <elf-binary>    # Run a RISC-V ELF binary
```

Run all riscv-tests compliance tests:
```bash
for f in tests/riscv-tests-bin/rv32ui-p-*; do
  result=$(cargo run --quiet -- "$f" 2>&1)
  echo "$(basename $f): $result"
done
```

Run a single riscv-test:
```bash
cargo run -- tests/riscv-tests-bin/rv32ui-p-add
```

## Architecture

Cargo workspace with three crates:

- **rvsim-core** — Pure computation, zero external dependencies. Contains CPU state (`Hart`), instruction decoder, execution engine, CSR file, memory trait, and trap definitions. This is the ISA implementation.
- **rvsim-mem** — Concrete memory implementations. `FlatMemory` is a `Vec<u8>` with a base address. Later will include bus routing to peripherals.
- **rvsim-cli** — Binary crate. Loads ELF via `goblin`, runs simulation, checks `tohost` symbol for pass/fail.

### Execution Flow

1. `Hart::step()` fetches instruction at `pc`, decodes to `Instruction` enum, advances `pc += 4`, then executes.
2. Traps are handled internally: on exception, `handle_trap()` saves `mepc`/`mcause` and jumps to `mtvec` — the CPU never stops on a trap.
3. Branches/jumps overwrite `pc` after the pre-increment. `AUIPC` uses `pc - 4` (the instruction's own address).

### Key Design Decisions

- **Instruction enum with named fields** — decoded once, matched exhaustively in `execute()`. Compiler verifies coverage.
- **Memory trait** returns `Result<_, Trap>` — access faults propagate naturally. Misaligned access is supported (no alignment traps in FlatMemory).
- **CSR file** uses `HashMap<u16, u32>`. `cycle`/`instret` CSRs are computed from `Hart` counters on read. Unknown CSR access raises `IllegalInstruction`.
- **tohost protocol** — riscv-tests signal completion by writing to a `tohost` memory address. Value 1 = pass, otherwise `(test_num << 1) | 1` = fail.

## Roadmap (not yet implemented)

M extension → A extension → C extension → F/D extensions → S/U privilege modes → Sv32 MMU → Bus → UART/CLINT/PLIC → OpenSBI → Linux boot.
