# Convenience targets for common rvsim workflows.
# Run `make help` for a list.

CARGO        ?= cargo
CARGO_FLAGS  ?=

RISCV_TESTS_DIR := tests/riscv-tests-bin
OPENSBI_ELF     := tests/opensbi-bin/fw_jump.elf
OPENSBI_DTB     := tests/opensbi-bin/rvsim.dtb

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

# Walks every ELF under tests/riscv-tests-bin (skipping .dump/.bin/.txt
# siblings), runs it, and prints only failures plus a PASS/FAIL summary.
# Exits non-zero if any test failed.
riscv-tests:
	@$(CARGO) build --quiet $(CARGO_FLAGS)
	@pass=0; fail=0; fails=""; \
	for f in $(RISCV_TESTS_DIR)/rv32*-p-*; do \
		case "$$f" in *.dump|*.bin|*.txt) continue;; esac; \
		result=$$($(CARGO) run --quiet $(CARGO_FLAGS) -- "$$f" 2>&1 | tail -1); \
		if [ "$$result" = "PASS" ]; then \
			pass=$$((pass+1)); \
		else \
			fail=$$((fail+1)); \
			fails="$$fails\n  $$(basename $$f): $$result"; \
		fi; \
	done; \
	if [ $$fail -gt 0 ]; then printf "Failures:$$fails\n"; fi; \
	echo "riscv-tests: PASS=$$pass FAIL=$$fail"; \
	[ $$fail -eq 0 ]

opensbi:
	$(CARGO) run --release $(CARGO_FLAGS) -- --dtb $(OPENSBI_DTB) $(OPENSBI_ELF)

clean:
	$(CARGO) clean
