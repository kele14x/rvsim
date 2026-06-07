# Convenience targets for common rvsim workflows.
# Run `make help` for a list.

CARGO        ?= cargo
CARGO_FLAGS  ?=

RISCV_TESTS_DIR    := tests/riscv-tests-bin
# Suite prefixes to run under `riscv-tests`. Override on the command line, e.g.
#   make riscv-tests RISCV_TEST_SUITES='rv32ui-p rv32uc-p'
RISCV_TEST_SUITES  ?= rv32ui-p rv32um-p rv32ua-p rv32uc-p rv32uf-p rv32ud-p rv32mi-p rv32si-p \
                       rv32ui-v rv32um-v rv32ua-v rv32uc-v rv32uf-v rv32ud-v
OPENSBI_ELF        := tests/opensbi-bin/fw_jump.elf
OPENSBI_DTB        := tests/device-tree-bin/rvsim.dtb

.PHONY: help build test clippy fmt check riscv-tests opensbi clean

help:
	@echo "Targets:"
	@echo "  build         Build all crates (debug)"
	@echo "  test          Run unit tests"
	@echo "  clippy        Lint with clippy"
	@echo "  fmt           Format with rustfmt"
	@echo "  check         build + test + clippy"
	@echo "  riscv-tests   Run the full riscv-tests suite; print failures + summary"
	@echo "  opensbi       Boot OpenSBI fw_jump.elf with the bundled DTB (release)"
	@echo "  clean         cargo clean"

build:
	$(CARGO) build $(CARGO_FLAGS)

test:
	$(CARGO) test $(CARGO_FLAGS)

clippy:
	$(CARGO) clippy $(CARGO_FLAGS) -- -D warnings

fmt:
	$(CARGO) fmt

check: build test clippy

# Walks the suites listed in RISCV_TEST_SUITES (skipping .dump/.bin/.txt
# siblings), runs each ELF, and prints only failures plus a PASS/FAIL summary.
# Exits non-zero if any test failed.
riscv-tests:
	@$(CARGO) build --quiet $(CARGO_FLAGS)
	@pass=0; fail=0; fails=""; \
	for suite in $(RISCV_TEST_SUITES); do \
		for f in $(RISCV_TESTS_DIR)/$$suite-*; do \
			case "$$f" in *.dump|*.bin|*.txt) continue;; esac; \
			[ -f "$$f" ] || continue; \
			result=$$($(CARGO) run --quiet $(CARGO_FLAGS) -- "$$f" 2>&1 | tail -1); \
			if [ "$$result" = "PASS" ]; then \
				pass=$$((pass+1)); \
			else \
				fail=$$((fail+1)); \
				fails="$$fails\n  $$(basename $$f): $$result"; \
			fi; \
		done; \
	done; \
	if [ $$fail -gt 0 ]; then printf "Failures:$$fails\n"; fi; \
	echo "riscv-tests: PASS=$$pass FAIL=$$fail"; \
	[ $$fail -eq 0 ]

opensbi:
	$(CARGO) run --release $(CARGO_FLAGS) -- --dtb $(OPENSBI_DTB) $(OPENSBI_ELF)

clean:
	$(CARGO) clean
