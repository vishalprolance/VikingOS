# Build and Test: VikingOS

This document describes the environment and process for building and validating VikingOS.

## Development Environment

### Host Requirements
- **Rust**: Nightly (via `rust-toolchain.toml`).
- **Python**: 3.8+ for build automation.
- **xorriso**: Required for ISO generation (install via WSL on Windows).
- **QEMU**: `qemu-system-x86_64` for testing.

### Build Commands
- **Kernel Only**: `cargo build --release` (inside `kernel/`)
- **ISO Build**: `python scripts/build_iso.py`
- **Automated Test**: `python scripts/test_qemu.py`

## Validation Pipeline

### 1. Compilation
- All kernel code must compile without warnings (excluding intentional `allow` attributes).

### 2. Automated Boot Test
- The `test_qemu.py` script boots the ISO in headless mode and verifies the "VikingOS" boot banner appears on the serial console.

### 3. Manual Verification
- Booting the ISO in QEMU or VMware Workstation.
- Verifying diagnostic output for memory maps and exception handlers.

### 4. Regression Testing
- New features must include tests in `main.rs` (temporary) or dedicated test modules that are executed during the boot sequence.
