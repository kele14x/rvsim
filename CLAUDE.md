# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RISC-V simulator (RV32GC + Sv32 MMU) in Rust, targeting Linux boot. Implements the full RV32GC ISA (IMAFDC extensions + Zicsr + Zifencei), supervisor mode, and Sv32 virtual memory.

## Build & Test Commands

```bash
cargo build                  # Build all crates
cargo test                   # Run unit tests (core + mem)
cargo clippy                 # Lint
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

Boot Linux:
```bash
cargo run -- tests/opensbi-bin/fw_jump.elf \
  --dtb tests/device-tree-bin/rvsim.dtb \
  --kernel tests/linux-bin/Image
```

## Architecture

Cargo workspace with three crates:

- **rvsim-core** — Pure computation, zero external dependencies. Contains CPU state (`Hart`), instruction decoder, execution engine, CSR file, MMU, memory trait, and trap definitions. This is the ISA implementation.
- **rvsim-mem** — Concrete memory implementations and peripheral devices. `FlatMemory` is a `Vec<u8>` with a base address. `Bus` routes MMIO accesses to CLINT (timer), PLIC (interrupt controller), and UART (serial console).
- **rvsim-cli** — Binary crate. Loads ELF via `goblin`, runs simulation, checks `tohost` symbol for pass/fail. Handles terminal raw mode for interactive Linux console.

### Execution Flow

1. `Hart::step()` fetches instruction at `pc` (2 or 4 bytes), decodes to `Instruction` enum, advances `pc` by instruction width, then executes.
2. Compressed (16-bit) instructions are expanded to their 32-bit equivalents before decoding.
3. Traps are handled internally: on exception, `handle_trap()` saves `mepc`/`mcause` and jumps to `mtvec` — the CPU never stops on a trap.
4. Branches/jumps overwrite `pc` after the pre-increment. `AUIPC` uses `pc - 4` (the instruction's own address).

### Key Design Decisions

- **Instruction enum with named fields** — decoded once, matched exhaustively in `execute()`. Compiler verifies coverage.
- **Memory trait** returns `Result<_, Trap>` — access faults propagate naturally. Misaligned access is supported (no alignment traps in FlatMemory).
- **CSR file** uses `[u32; 4096]` array for fast indexed access. `cycle`/`instret`/`time` CSRs are computed from `Hart` counters on read.
- **Compressed instruction expansion** — `expand_compressed()` converts 16-bit RVC encodings to canonical 32-bit form before decode, keeping the decoder simple.
- **tohost protocol** — riscv-tests signal completion by writing to a `tohost` memory address. Value 1 = pass, otherwise `(test_num << 1) | 1` = fail.

## Pre-Commit Checklist

Before committing any code change, always run these three checks and fix any issues:

```bash
cargo fmt -- --check    # Format — run without --check to auto-fix
cargo clippy            # Lint warnings
cargo test              # Unit tests must pass
```

If `cargo fmt -- --check` reports diffs, run `cargo fmt` to apply formatting, then re-check. Clippy warnings and test failures must be resolved before the commit.

## Status

RV32GC implementation is complete and boots Linux via OpenSBI. All ISA extensions (IMAFDC + Zicsr + Zifencei) are implemented and tested. Supervisor mode with Sv32 MMU enables full Linux operation including virtual memory, page faults, and user-space execution.
