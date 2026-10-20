# VikingOS Build System
# =====================
#
# Commands:
#   make kernel   — Compile the kernel
#   make iso      — Build kernel + assemble bootable ISO
#   make test     — Build ISO + run QEMU boot test
#   make run      — Build ISO + launch interactive QEMU
#   make clean    — Remove all build artifacts
#
# Requirements:
#   - Rust nightly (see rust-toolchain.toml)
#   - Python 3.8+
#   - Git (for cloning Limine)
#   - xorriso (via WSL on Windows)
#   - QEMU (for testing)

# ── Paths ─────────────────────────────────────────────────────────────

CARGO       = cargo
PYTHON      = python
KERNEL_DIR  = kernel
TARGET      = x86_64-unknown-none
PROFILE     = release
KERNEL_ELF  = $(KERNEL_DIR)/target/$(TARGET)/$(PROFILE)/viking_kernel
ISO_OUTPUT  = build/VikingOS.iso

# ── Phony Targets ─────────────────────────────────────────────────────

.PHONY: kernel iso test run clean help

# ── Default ───────────────────────────────────────────────────────────

help:
	@echo.
	@echo  VikingOS Build System
	@echo  =====================
	@echo.
	@echo  make kernel   — Compile the kernel
	@echo  make iso      — Build kernel + assemble bootable ISO
	@echo  make test     — Build ISO + run QEMU boot test
	@echo  make run      — Build ISO + launch interactive QEMU
	@echo  make clean    — Remove all build artifacts
	@echo.

# ── Kernel ────────────────────────────────────────────────────────────

kernel:
	@echo [BUILD ] Compiling VikingOS kernel...
	cd $(KERNEL_DIR) && $(CARGO) build --release
	@echo [  OK  ] Kernel compiled: $(KERNEL_ELF)

# ── ISO ───────────────────────────────────────────────────────────────

iso: kernel
	@echo [BUILD ] Assembling ISO...
	$(PYTHON) scripts/build_iso.py --kernel $(KERNEL_ELF) --output $(ISO_OUTPUT)

# ── Test ──────────────────────────────────────────────────────────────

test: iso
	@echo [TEST  ] Running QEMU boot test...
	$(PYTHON) scripts/test_qemu.py --iso $(ISO_OUTPUT) --timeout 15

# ── Interactive Run ───────────────────────────────────────────────────

run: iso
	$(PYTHON) scripts/test_qemu.py --iso $(ISO_OUTPUT) --interactive

# ── Clean ─────────────────────────────────────────────────────────────

clean:
	@echo [CLEAN ] Removing build artifacts...
	cd $(KERNEL_DIR) && $(CARGO) clean
	@if exist build rmdir /s /q build
	@echo [  OK  ] Clean complete
