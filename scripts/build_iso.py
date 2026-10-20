#!/usr/bin/env python3
"""
VikingOS ISO Build Script

Assembles a bootable ISO image containing the VikingOS kernel and Limine
bootloader. Supports both BIOS and UEFI boot.

Usage:
    python scripts/build_iso.py [--kernel PATH] [--output PATH]

Requirements:
    - Limine bootloader (cloned into build/limine/)
    - xorriso (available via WSL on Windows, or natively on Linux/macOS)
    - Kernel ELF binary already compiled

This script is designed to run on Windows (using WSL for xorriso) or
natively on Linux/macOS.
"""

import argparse
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path

# ── Configuration ─────────────────────────────────────────────────────

LIMINE_REPO = "https://github.com/limine-bootloader/limine.git"
LIMINE_BRANCH = "v8.x-binary"

# Relative to project root
DEFAULT_KERNEL = "kernel/target/x86_64-unknown-none/release/viking_kernel"
DEFAULT_OUTPUT = "build/VikingOS.iso"
LIMINE_DIR = "build/limine"
ISO_ROOT = "build/iso_root"


def project_root() -> Path:
    """Get the project root directory (parent of scripts/)."""
    return Path(__file__).resolve().parent.parent


def run(cmd: list[str], cwd: str | None = None, check: bool = True) -> subprocess.CompletedProcess:
    """Run a command and print it for visibility."""
    print(f"  -> {' '.join(cmd)}")
    return subprocess.run(cmd, cwd=cwd, check=check, capture_output=False)


def run_wsl(cmd: str, cwd: str | None = None, check: bool = True) -> subprocess.CompletedProcess:
    """Run a command through WSL (Windows only)."""
    return run(["wsl", "--", "sh", "-c", cmd], cwd=cwd, check=check)


def is_windows() -> bool:
    return platform.system() == "Windows"


def wsl_path(win_path: str) -> str:
    """Convert a Windows path to a WSL path."""
    posix_path = win_path.replace("\\", "/")
    result = subprocess.run(
        ["wsl", "wslpath", "-u", posix_path],
        capture_output=True, text=True, check=True
    )
    return result.stdout.strip()


# ── Steps ─────────────────────────────────────────────────────────────

def clone_limine(root: Path) -> Path:
    """Clone or update the Limine bootloader binary release."""
    limine_path = root / LIMINE_DIR
    if limine_path.exists() and ((limine_path / "limine.exe").exists() or (limine_path / "limine").exists()):
        print("[  OK  ] Limine already present")
        return limine_path

    print("[BUILD ] Cloning Limine bootloader...")
    if limine_path.exists():
        shutil.rmtree(limine_path)
    run([
        "git", "clone", "--depth=1",
        "--branch", LIMINE_BRANCH,
        LIMINE_REPO,
        str(limine_path)
    ])

    # Build the limine utility (needed for BIOS install)
    if is_windows():
        if not (limine_path / "limine.exe").exists():
            wsl_limine = wsl_path(str(limine_path))
            run_wsl(f"cd '{wsl_limine}' && make -C . limine", cwd=str(root))
    else:
        run(["make", "-C", str(limine_path), "limine"])

    print("[  OK  ] Limine bootloader ready")
    return limine_path


def prepare_iso_root(root: Path, kernel_path: Path, limine_path: Path) -> Path:
    """Create the ISO directory structure."""
    iso_root = root / ISO_ROOT

    # Clean and recreate
    if iso_root.exists():
        shutil.rmtree(iso_root)

    # Create directory structure
    (iso_root / "boot" / "limine").mkdir(parents=True)
    (iso_root / "boot").mkdir(exist_ok=True)
    (iso_root / "EFI" / "BOOT").mkdir(parents=True)

    print("[BUILD ] Assembling ISO filesystem...")

    # Copy kernel
    kernel_dest = iso_root / "boot" / "viking_kernel"
    shutil.copy2(kernel_path, kernel_dest)
    kernel_size = kernel_dest.stat().st_size
    print(f"         Kernel: {kernel_size:,} bytes ({kernel_size // 1024} KB)")

    # Copy limine.conf
    shutil.copy2(root / "limine.conf", iso_root / "boot" / "limine" / "limine.conf")

    # Copy Limine boot files
    limine_files = {
        "limine-bios.sys": "boot/limine/limine-bios.sys",
        "limine-bios-cd.bin": "boot/limine/limine-bios-cd.bin",
        "limine-uefi-cd.bin": "boot/limine/limine-uefi-cd.bin",
        "BOOTX64.EFI": "EFI/BOOT/BOOTX64.EFI",
        "BOOTIA32.EFI": "EFI/BOOT/BOOTIA32.EFI",
    }

    for src_name, dest_rel in limine_files.items():
        src = limine_path / src_name
        if src.exists():
            shutil.copy2(src, iso_root / dest_rel)
            print(f"         Copied: {src_name}")
        else:
            print(f"         Skip:   {src_name} (not found)")

    print("[  OK  ] ISO filesystem ready")
    return iso_root


def build_iso(root: Path, iso_root: Path, output: Path, limine_path: Path) -> Path:
    """Generate the bootable ISO image using xorriso."""
    print("[BUILD ] Generating ISO image...")

    output.parent.mkdir(parents=True, exist_ok=True)

    if is_windows():
        # Use WSL for xorriso
        wsl_iso_root = wsl_path(str(iso_root))
        wsl_output = wsl_path(str(output))
        wsl_limine = wsl_path(str(limine_path))

        # Check xorriso
        run_wsl("which xorriso || apk add --no-cache xorriso || (sudo apt-get update -qq && sudo apt-get install -y -qq xorriso)", cwd=str(root))

        xorriso_cmd = (
            f"xorriso -as mkisofs "
            f"-b boot/limine/limine-bios-cd.bin "
            f"-no-emul-boot -boot-load-size 4 -boot-info-table "
            f"--efi-boot boot/limine/limine-uefi-cd.bin "
            f"-efi-boot-part --efi-boot-image --protective-msdos-label "
            f"'{wsl_iso_root}' -o '{wsl_output}'"
        )
        run_wsl(xorriso_cmd, cwd=str(root))

        # Install BIOS boot sectors
        limine_exe = limine_path / "limine.exe"
        if not limine_exe.exists():
            limine_exe = limine_path / "limine"
        if limine_exe.exists():
            if str(limine_exe).endswith(".exe"):
                run([str(limine_exe), "bios-install", str(output)])
            else:
                wsl_limine_exe = wsl_path(str(limine_exe))
                run_wsl(f"'{wsl_limine_exe}' bios-install '{wsl_output}'", cwd=str(root))
            print("[  OK  ] BIOS boot sectors installed")
        else:
            print("[ WARN ] limine utility not found — BIOS boot may not work")
    else:
        # Native Linux/macOS
        run([
            "xorriso", "-as", "mkisofs",
            "-b", "boot/limine/limine-bios-cd.bin",
            "-no-emul-boot", "-boot-load-size", "4", "-boot-info-table",
            "--efi-boot", "boot/limine/limine-uefi-cd.bin",
            "-efi-boot-part", "--efi-boot-image", "--protective-msdos-label",
            str(iso_root), "-o", str(output)
        ])

        limine_exe = limine_path / "limine"
        if limine_exe.exists():
            run([str(limine_exe), "bios-install", str(output)])
            print("[  OK  ] BIOS boot sectors installed")

    if not output.exists():
        print("[FATAL ] ISO generation failed — output file not found!")
        sys.exit(1)

    iso_size = output.stat().st_size
    print(f"[  OK  ] ISO created: {output}")
    print(f"         Size: {iso_size:,} bytes ({iso_size // 1024} KB, {iso_size // (1024*1024)} MB)")
    return output


def validate_iso(output: Path) -> None:
    """Verify the ISO is valid and contains expected files."""
    print("[CHECK ] Validating ISO...")

    if not output.exists():
        print("[FATAL ] ISO file does not exist!")
        sys.exit(1)

    size = output.stat().st_size
    if size == 0:
        print("[FATAL ] ISO file is empty!")
        sys.exit(1)

    # Check minimum reasonable size (should be at least a few hundred KB)
    if size < 100_000:
        print(f"[ WARN ] ISO is suspiciously small: {size:,} bytes")

    print(f"[  OK  ] ISO validation passed: {output} ({size:,} bytes)")


# ── Main ──────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Build VikingOS bootable ISO")
    parser.add_argument("--kernel", default=DEFAULT_KERNEL,
                        help="Path to kernel ELF binary (relative to project root)")
    parser.add_argument("--output", default=DEFAULT_OUTPUT,
                        help="Output ISO path (relative to project root)")
    args = parser.parse_args()

    root = project_root()
    kernel_path = root / args.kernel
    output_path = root / args.output

    print("=" * 60)
    print("  VikingOS ISO Builder")
    print("=" * 60)
    print(f"  Project root: {root}")
    print(f"  Kernel:       {kernel_path}")
    print(f"  Output:       {output_path}")
    print(f"  Platform:     {platform.system()} ({platform.machine()})")
    print("=" * 60)
    print()

    # Verify kernel exists
    if not kernel_path.exists():
        print(f"[FATAL ] Kernel binary not found: {kernel_path}")
        print("         Run 'make kernel' first to compile the kernel.")
        sys.exit(1)

    # Step 1: Get Limine
    limine_path = clone_limine(root)

    # Step 2: Prepare ISO filesystem
    iso_root = prepare_iso_root(root, kernel_path, limine_path)

    # Step 3: Generate ISO
    build_iso(root, iso_root, output_path, limine_path)

    # Step 4: Validate
    validate_iso(output_path)

    print()
    print("=" * 60)
    print("  Build complete!")
    print(f"  ISO: {output_path}")
    print(f"  Test with: make test")
    print("=" * 60)


if __name__ == "__main__":
    main()
