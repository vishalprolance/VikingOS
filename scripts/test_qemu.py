#!/usr/bin/env python3
"""
VikingOS QEMU Boot Test Script

Launches the VikingOS ISO in QEMU, captures serial output, and verifies
that the kernel boots successfully by checking for the expected banner.

Usage:
    python scripts/test_qemu.py [--iso PATH] [--timeout SECONDS]

Exit codes:
    0 — Boot test passed (banner found in serial output)
    1 — Boot test failed (banner not found or QEMU error)

Requirements:
    - QEMU (qemu-system-x86_64) installed and in PATH
    - VikingOS ISO already built
"""

import argparse
import os
import platform
import subprocess
import sys
import time
import threading
from pathlib import Path

# ── Configuration ─────────────────────────────────────────────────────

DEFAULT_ISO = "build/VikingOS.iso"
DEFAULT_TIMEOUT = 15  # seconds
EXPECTED_BANNER = "VikingOS"
EXPECTED_SUCCESS = "Boot successful"

QEMU_CMD = "qemu-system-x86_64"

QEMU_ARGS = [
    "-cdrom", "{iso}",
    "-serial", "stdio",
    "-display", "none",
    "-m", "256M",
    "-no-reboot",
    "-no-shutdown",
]


def project_root() -> Path:
    return Path(__file__).resolve().parent.parent


def find_qemu() -> str:
    """Locate qemu-system-x86_64 executable."""
    # Common Windows install paths
    if platform.system() == "Windows":
        common_paths = [
            r"C:\Program Files\qemu\qemu-system-x86_64.exe",
            r"C:\Program Files (x86)\qemu\qemu-system-x86_64.exe",
        ]
        for p in common_paths:
            if os.path.exists(p):
                return p

    # Check PATH
    result = subprocess.run(
        ["where" if platform.system() == "Windows" else "which", QEMU_CMD],
        capture_output=True, text=True
    )
    if result.returncode == 0:
        return result.stdout.strip().split('\n')[0]

    return QEMU_CMD  # Hope it's in PATH


def run_qemu_test(iso_path: Path, timeout: int) -> tuple[bool, str]:
    """
    Run QEMU with the ISO and capture serial output.

    Returns (success, captured_output).
    """
    qemu = find_qemu()
    args = [qemu] + [
        arg.format(iso=str(iso_path)) for arg in QEMU_ARGS
    ]

    print(f"  Command: {' '.join(args)}")
    print(f"  Timeout: {timeout}s")
    print()

    output_lines: list[str] = []
    found_banner = False
    found_success = False

    try:
        proc = subprocess.Popen(
            args,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        )

        def read_output():
            nonlocal found_banner, found_success
            for line in iter(proc.stdout.readline, ''):
                stripped = line.rstrip('\r\n')
                output_lines.append(stripped)
                print(f"  [SERIAL] {stripped}")

                if EXPECTED_BANNER in line:
                    found_banner = True
                if EXPECTED_SUCCESS in line:
                    found_success = True
                    # Give a moment for remaining output, then we can stop
                    time.sleep(1)
                    proc.terminate()
                    return

        reader = threading.Thread(target=read_output, daemon=True)
        reader.start()
        reader.join(timeout=timeout)

        # Kill QEMU if still running
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()

    except FileNotFoundError:
        print(f"[FATAL ] QEMU not found: {qemu}")
        print("         Install QEMU and ensure qemu-system-x86_64 is in PATH.")
        return False, ""
    except Exception as e:
        print(f"[FATAL ] QEMU execution error: {e}")
        return False, ""

    full_output = '\n'.join(output_lines)
    success = found_banner  # Minimum requirement: banner was printed
    return success, full_output


def main():
    parser = argparse.ArgumentParser(description="VikingOS QEMU boot test")
    parser.add_argument("--iso", default=DEFAULT_ISO,
                        help="Path to ISO (relative to project root)")
    parser.add_argument("--timeout", type=int, default=DEFAULT_TIMEOUT,
                        help="Timeout in seconds")
    parser.add_argument("--interactive", action="store_true",
                        help="Run with display (not headless)")
    args = parser.parse_args()

    root = project_root()
    iso_path = root / args.iso

    print("=" * 60)
    print("  VikingOS QEMU Boot Test")
    print("=" * 60)
    print(f"  ISO: {iso_path}")
    print()

    if not iso_path.exists():
        print(f"[FATAL ] ISO not found: {iso_path}")
        print("         Run 'make iso' first.")
        sys.exit(1)

    if args.interactive:
        # Interactive mode — just launch QEMU with display
        qemu = find_qemu()
        interactive_args = [
            qemu,
            "-cdrom", str(iso_path),
            "-serial", "stdio",
            "-m", "256M",
        ]
        print(f"  Launching interactive QEMU...")
        print(f"  Command: {' '.join(interactive_args)}")
        subprocess.run(interactive_args)
        return

    # Automated test mode
    success, output = run_qemu_test(iso_path, args.timeout)

    print()
    print("=" * 60)
    if success:
        print("  [PASS] BOOT TEST PASSED")
        print(f"    Banner '{EXPECTED_BANNER}' found in serial output")
    else:
        print("  [FAIL] BOOT TEST FAILED")
        if not output:
            print("    No serial output captured - QEMU may not have started")
        else:
            print(f"    Banner '{EXPECTED_BANNER}' NOT found in serial output")
            print()
            print("  Captured output:")
            for line in output.split('\n')[-20:]:
                print(f"    {line}")
    print("=" * 60)

    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
