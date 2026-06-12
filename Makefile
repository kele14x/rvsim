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
LINUX_IMAGE        := tests/linux-bin/Image

.PHONY: help build test clippy fmt check riscv-tests riscv-tests-clean opensbi linux clean

help:
	@echo "Targets:"
	@echo "  build              Build all crates (debug)"
	@echo "  test               Run unit tests"
	@echo "  clippy             Lint with clippy"
	@echo "  fmt                Format with rustfmt"
	@echo "  check              build + test + clippy"
	@echo "  riscv-tests        Run the full riscv-tests suite (auto-builds on first run)"
	@echo "  riscv-tests-clean  Remove built test binaries to force a fresh rebuild"
	@echo "  opensbi            Boot OpenSBI fw_jump.elf with the bundled DTB (release)"
	@echo "  linux              Boot Linux (OpenSBI + kernel + DTB, release)"
	@echo "  clean              cargo clean"

build:
	$(CARGO) build $(CARGO_FLAGS)

test:
	$(CARGO) test $(CARGO_FLAGS)

clippy:
	$(CARGO) clippy $(CARGO_FLAGS) -- -D warnings

fmt:
	$(CARGO) fmt

check: build test clippy

# --- riscv-tests ----------------------------------------------------------

RISCV_TESTS_STAMP := $(RISCV_TESTS_DIR)/.built

# Sentinel file — runs fetch + build once, skipped on subsequent invocations
# unless riscv-tests-clean was run.
$(RISCV_TESTS_STAMP):
	@# Fetch source if not already present (keeps stamp independent of PHONY fetch).
	@if [ ! -f tests/riscv-tests/configure ]; then \
		git submodule update --init --recursive --depth 1 tests/riscv-tests; \
	fi
	cd tests/riscv-tests && autoconf && ./configure --with-xlen=32
	$(MAKE) -C tests/riscv-tests
	@mkdir -p $(RISCV_TESTS_DIR)
	@cp tests/riscv-tests/isa/rv32ui-* $(RISCV_TESTS_DIR)/
	@cp tests/riscv-tests/isa/rv32um-* $(RISCV_TESTS_DIR)/
	@cp tests/riscv-tests/isa/rv32ua-* $(RISCV_TESTS_DIR)/
	@cp tests/riscv-tests/isa/rv32uc-* $(RISCV_TESTS_DIR)/
	@cp tests/riscv-tests/isa/rv32uf-* $(RISCV_TESTS_DIR)/
	@cp tests/riscv-tests/isa/rv32ud-* $(RISCV_TESTS_DIR)/
	@cp tests/riscv-tests/isa/rv32mi-* $(RISCV_TESTS_DIR)/
	@cp tests/riscv-tests/isa/rv32si-* $(RISCV_TESTS_DIR)/
	@touch $@
	@echo "riscv-tests binaries copied to $(RISCV_TESTS_DIR)/"

# Remove built artifacts and copied binaries so the next riscv-tests
# run rebuilds from scratch.
riscv-tests-clean:
	@# Clean the riscv-tests build output (isa/ binaries).
	@if [ -f tests/riscv-tests/Makefile ]; then \
		$(MAKE) -C tests/riscv-tests clean; \
	fi
	@# Remove autoconf / configure artifacts.
	@rm -f tests/riscv-tests/Makefile
	@rm -f tests/riscv-tests/config.status
	@rm -f tests/riscv-tests/config.log
	@rm -f tests/riscv-tests/configure
	@rm -rf tests/riscv-tests/autom4te.cache
	@# Remove copied test binaries and sentinel.
	@rm -f $(RISCV_TESTS_DIR)/rv32ui-*
	@rm -f $(RISCV_TESTS_DIR)/rv32um-*
	@rm -f $(RISCV_TESTS_DIR)/rv32ua-*
	@rm -f $(RISCV_TESTS_DIR)/rv32uc-*
	@rm -f $(RISCV_TESTS_DIR)/rv32uf-*
	@rm -f $(RISCV_TESTS_DIR)/rv32ud-*
	@rm -f $(RISCV_TESTS_DIR)/rv32mi-*
	@rm -f $(RISCV_TESTS_DIR)/rv32si-*
	@rm -f $(RISCV_TESTS_STAMP)
	@echo "riscv-tests cleaned"

# Run the full riscv-tests suite.  Automatically fetches and builds the test
# binaries if they haven't been built yet (or after a clean).
# Exits non-zero if any test failed.
riscv-tests: $(RISCV_TESTS_STAMP)
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

linux:
	$(CARGO) run --release $(CARGO_FLAGS) -- \
		$(OPENSBI_ELF) \
		--dtb $(OPENSBI_DTB) \
		--kernel $(LINUX_IMAGE)

clean:
	$(CARGO) clean
