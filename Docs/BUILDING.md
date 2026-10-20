# Building VikingOS

This document explains how to set up your development environment and
build VikingOS from source.

## Host Requirements

| Tool | Version | Purpose |
|---|---|---|
| **Rust** (nightly) | See `rust-toolchain.toml` | Kernel compilation |
| **Python** | 3.8+ | Build & test automation scripts |
| **Git** | 2.x+ | Source control, Limine download |
| **xorriso** | 1.5+ | ISO image generation |
| **QEMU** | 7.0+ | x86-64 virtual machine testing |

### Windows-Specific Notes

- **Rust**: Install via [rustup](https://rustup.rs/). The `rust-toolchain.toml`
  will automatically select the correct nightly version and components.
- **xorriso**: Not natively available on Windows. Install via WSL:
  ```bash
  wsl sudo apt-get install -y xorriso
  ```
- **QEMU**: Download from [qemu.org](https://www.qemu.org/download/#windows)
  or install via `winget install QEMU.QEMU`.
- **Make**: If `make` is not available, use the equivalent commands listed below.

### Installing Rust

If Rust is not installed:

```powershell
# Windows (PowerShell)
Invoke-WebRequest -Uri https://win.rustup.rs/x86_64 -OutFile rustup-init.exe
.\rustup-init.exe -y --default-toolchain nightly
# Restart your shell after installation
```

The `rust-toolchain.toml` in the repository root ensures the correct
nightly version, target, and components are used automatically.

## Build Commands

### Using Make

```bash
make kernel   # Compile the kernel
make iso      # Build kernel + assemble bootable ISO
make test     # Build ISO + run automated QEMU boot test
make run      # Build ISO + launch interactive QEMU session
make clean    # Remove all build artifacts
```

### Without Make (manual commands)

```powershell
# Compile kernel
cd kernel
cargo build --release

# Build ISO
cd ..
python scripts/build_iso.py

# Run QEMU test
python scripts/test_qemu.py

# Clean
cd kernel && cargo clean && cd ..
Remove-Item -Recurse -Force build -ErrorAction SilentlyContinue
```

## Build Outputs

| Artifact | Path | Description |
|---|---|---|
| Kernel ELF | `kernel/target/x86_64-unknown-none/release/viking_kernel` | Raw kernel binary |
| Bootable ISO | `build/VikingOS.iso` | BIOS+UEFI bootable ISO image |

## First-Time Build

On the first build, the ISO builder will:

1. Clone the Limine bootloader (~30 seconds)
2. Build the Limine utility (requires WSL `make` on Windows)
3. Compile the kernel (~60 seconds first time, faster after)
4. Assemble the ISO

Subsequent builds are incremental and much faster.

## Troubleshooting

### "rust-src component not found"

The `rust-toolchain.toml` should handle this automatically. If not:

```bash
rustup component add rust-src llvm-tools --toolchain nightly
```

### "linker not found"

Ensure the nightly Rust toolchain is active:

```bash
rustup default nightly
rustup target add x86_64-unknown-none
```

### "xorriso not found"

On Windows, install via WSL:

```bash
wsl sudo apt-get update && sudo apt-get install -y xorriso
```

### Cargo build fails with "can't find crate for `core`"

Ensure `build-std` is configured in `.cargo/config.toml` (it should be).
Also verify `rust-src` component:

```bash
rustup component add rust-src
```

## Dependency Licenses

| Dependency | License | Purpose |
|---|---|---|
| Rust compiler | MIT/Apache-2.0 | Compilation toolchain |
| `limine` crate | BSD-2-Clause | Boot protocol types |
| Limine bootloader | BSD-2-Clause | System bootloader |
| xorriso | GPL-3.0 | ISO generation (build tool only) |
| QEMU | GPL-2.0 | Testing (not shipped) |
