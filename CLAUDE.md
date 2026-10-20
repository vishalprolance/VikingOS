# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

### Common Commands
- **Compile Kernel**: `make kernel`
- **Build Bootable ISO**: `make iso`
- **Run Automated Boot Test**: `make test` (Boots in QEMU headless, verifies boot banner via serial)
- **Run Interactively in QEMU**: `make run`
- **Clean Build Artifacts**: `make clean`

### Manual Build (Without Make)
- **Compile Kernel**: `cd kernel && cargo build --release`
- **Build ISO**: `python scripts/build_iso.py`
- **Run QEMU Test**: `python scripts/test_qemu.py`

## Architecture Overview

VikingOS is an independent x86-64 operating system written in `no_std` Rust, targeting `x86_64-unknown-none`.

### High-Level Boot Flow
1. **Firmware (UEFI/BIOS)** $\rightarrow$ **Limine Bootloader**
2. **Limine** sets up higher-half paging (Kernel at `0xFFFFFFFF80000000`) and provides boot information (memory map, framebuffer, HHDM).
3. **Kernel Entry** (`_start` in `kernel/src/main.rs`) initializes the serial console and prints diagnostics.

### Key Components
- **Kernel**: Located in `kernel/`. A bare-metal Rust crate using the `limine` crate for boot protocol interaction.
- **Serial Driver** (`kernel/src/serial.rs`): Implements `core::fmt::Write` for COM1 (I/O port `0x3F8`), providing the primary diagnostic output.
- **CPU Core (Milestone 2)**:
    - **GDT/TSS**: Manages segments (Ring 0/3) and provides a dedicated Double Fault stack (IST1) to prevent triple faults.
    - **IDT**: Handles CPU exceptions via assembly trampolines that save register state and dispatch to Rust handlers.
- **Memory Layout**:
    - **Kernel Code/Data**: Higher-half virtual addresses starting at `0xFFFFFFFF80000000`.
    - **HHDM**: Direct map of all physical memory starting at `0xFFFF800000000000`.

### Target Configuration
- **Target**: `x86_64-unknown-none`
- **Toolchain**: Rust Nightly (pinned in `rust-toolchain.toml`)
- **Linker**: `rust-lld` via `kernel/linker.ld`
- **Panic Strategy**: `abort` (prints to serial, then halts)
