# VikingOS

An independent, open-source, x86-64 operating system written in Rust.

> **Status**: Full Implementation Complete (Milestones 1 to 12) — Foundation, Memory, Multitasking, GUI Window Compositor, System Applications, MBR Partitioning, VikingFS, & Disk Installer  
> VikingOS is experimental software. It is not production-ready.

## What is VikingOS?

VikingOS is an operating system built from scratch. It does **not** use the
Linux kernel, is **not** a Linux distribution, and does **not** build on any
existing OS. The kernel and all system components are original.

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (nightly — managed by `rust-toolchain.toml`)
- [QEMU](https://www.qemu.org/download/) (`qemu-system-x86_64`) or VMware Workstation
- [Git](https://git-scm.com/)
- Python 3.8+
- `xorriso` (via WSL on Windows: `wsl sudo apt-get install -y xorriso` or `apk add xorriso`)

### Build & Test

```bash
# Compile the kernel
make kernel

# Build a bootable ISO
make iso

# Boot in QEMU and verify (automated)
make test

# Boot in QEMU interactively
make run
```

### Without Make (Windows PowerShell)

```powershell
cd kernel && cargo build --release && cd ..
python scripts/build_iso.py
python scripts/test_qemu.py
```

## Project Structure

```
VikingOS/
├── kernel/              Rust kernel (no_std, bare-metal)
│   ├── src/
│   │   ├── main.rs      Kernel entry point & subsystem orchestrator
│   │   ├── serial.rs    16550 UART COM1 serial port driver
│   │   ├── thread.rs    Thread model & thread states (Ready/Running/Blocked/Terminated)
│   │   ├── arch/        x86-64 low-level CPU architecture
│   │   │   ├── gdt.rs   Global Descriptor Table & Task State Segment (TSS)
│   │   │   ├── idt.rs   Interrupt Descriptor Table (256 vectors)
│   │   │   ├── exceptions.rs 32 CPU exception handlers & ISR stubs
│   │   │   ├── io.rs    Safe port I/O wrappers (inb, outb, inw, outw, inl, outl)
│   │   │   ├── apic.rs  Local APIC driver & periodic timer
│   │   │   ├── ioapic.rs I/O APIC routing & hardware IRQ mapping
│   │   │   ├── interrupts.rs Interrupt enable/disable (sti/cli)
│   │   │   ├── interrupt_dispatcher.rs Hardware IRQ dispatcher (Timer 32, Keyboard 33)
│   │   │   └── context.rs Low-level AMD64 context switching assembly
│   │   ├── memory/      Memory management subsystem
│   │   │   ├── pmm.rs   Bitmap physical frame allocator (4 KiB frames)
│   │   │   ├── vmm.rs   4-level paging (PML4, PDPT, PD, PT) & address translation
│   │   │   └── heap.rs  Dynamic kernel heap allocator with `alloc` crate integration
│   │   ├── drivers/     Hardware and peripheral device drivers
│   │   │   ├── keyboard.rs PS/2 keyboard driver & scancode decoder
│   │   │   ├── mouse.rs PS/2 mouse driver with software arrow cursor
│   │   │   ├── pci.rs   PCI bus discovery & configuration space access
│   │   │   ├── framebuffer.rs Linear framebuffer abstraction
│   │   │   ├── font.rs  8x8 ASCII bitmap font renderer
│   │   │   ├── console.rs Graphical on-screen text console with scrolling
│   │   │   └── gui.rs   2D Canvas, Window Manager, Windows & Taskbar
│   │   ├── sync/        Concurrency and synchronization primitives
│   │   │   ├── spinlock.rs SpinLock with RAII guard
│   │   │   └── scheduler.rs Preemptive Round-Robin scheduler
│   │   ├── storage/     Storage device abstractions & disk partitioning
│   │   │   ├── mod.rs   BlockDevice trait, RamBlockDevice, error types
│   │   │   ├── virtio_block.rs VirtIO block device driver
│   │   │   └── partition.rs MBR partition table generator & validator
│   │   ├── fs/          Filesystem implementations
│   │   │   ├── vfs.rs   Virtual File System abstractions (Inode, FileHandle)
│   │   │   ├── ramdisk.rs In-memory read-only ramdisk filesystem
│   │   │   └── vikingfs.rs Persistent block filesystem (Superblock VIKI)
│   │   ├── apps/        Built-in system applications
│   │   │   ├── terminal.rs Viking Prompt interactive shell
│   │   │   ├── file_explorer.rs Graphical file manager
│   │   │   ├── vikingpad.rs Text editor application
│   │   │   └── task_manager.rs System & thread performance monitor
│   │   ├── installer/   Guided OS installation engine & wizard
│   │   ├── process/     Process abstraction & address space management
│   │   └── syscall/     x86_64 fast syscall/sysret dispatcher & MSR configuration
│   ├── linker.ld        Kernel linker script (higher-half: 0xFFFFFFFF80000000)
│   └── Cargo.toml       Kernel manifest
├── scripts/
│   ├── build_iso.py     Automated ISO builder (El Torito BIOS + UEFI bootable)
│   └── test_qemu.py     Automated QEMU verification runner
├── Docs/
│   ├── ARCHITECTURE.md  Technical system architecture
│   ├── BUILDING.md      Detailed build instructions & prerequisites
│   ├── TODO.md          Master roadmap checklist
│   ├── PROJECT_STATUS.md Milestone verification matrix
│   ├── KNOWN_LIMITATIONS.md Current gaps and upcoming features
│   └── VMWARE_TESTING.md VMware Workstation setup guide
├── limine.conf          Limine bootloader configuration
├── Makefile             Project build targets
├── rust-toolchain.toml  Nightly toolchain specification
└── .cargo/config.toml   Target definition & compiler flags
```

## Architecture

- **Bootloader**: [Limine](https://github.com/limine-bootloader/limine) (Protocol Rev 3, BIOS + UEFI)
- **Target**: `x86_64-unknown-none` (bare-metal, higher-half kernel)
- **Language**: Rust (`no_std`) with `alloc` support via custom heap
- **Diagnostics**: COM1 serial port (115200 baud, 8N1) and on-screen graphical console
- **ISO**: Hybrid El Torito bootable image for BIOS and UEFI via `xorriso` + `limine`

See [Docs/ARCHITECTURE.md](Docs/ARCHITECTURE.md) for full details, and [Docs/TODO.md](Docs/TODO.md) for the active step-by-step feature installation checklist.

## Completed Milestones (1 through 12)

The kernel provides a complete, operational operating system implementation:

- ✅ **M1: Boot & Diagnostics** — Limine boot (BIOS + UEFI), COM1 serial diagnostics, memory map enumeration, HHDM discovery
- ✅ **M2: CPU Core & Exceptions** — GDT, TSS (IST1 Double Fault stack), 256-entry IDT, 32 CPU exception handlers, breakpoint trap recovery
- ✅ **M3: Memory Management** — Physical frame allocator (PMM bitmap), 4-level paging (VMM), kernel dynamic heap allocator (`alloc` crate: `Vec`, `String`, `Box`, `Arc`)
- ✅ **M4: Hardware Interrupts & Timers** — Local APIC initialization, 100 Hz APIC timer, I/O APIC redirection table IRQ routing, atomic tick counter
- ✅ **M5: Input Devices** — PS/2 keyboard driver, scancode-to-ASCII translation, event handling
- ✅ **M6: Graphics & Text Console** — Linear framebuffer, 8x8 font rendering, scrolling graphical text console, 2D GUI Canvas, Taskbar and Desktop windows
- ✅ **M7: Multitasking & Scheduler** — Low-level AMD64 context switching, preemptive round-robin scheduler, kernel thread creation and preemption
- ✅ **M8: Storage & Virtual Filesystem** — `BlockDevice` trait, VirtIO block driver, VFS abstractions (`Inode`, `FileHandle`), mounted Ramdisk filesystem
- ✅ **M9: User-Mode & System Calls** — Ring 3 user transition stack frame setup, `IA32_STAR`/`IA32_LSTAR`/`IA32_FMASK` MSR configuration, fast `syscall` dispatcher
- ✅ **M10: Window Server & Desktop GUI Compositor** — 2D canvas primitives, draggable multi-window manager, Z-ordering, Taskbar with RAM/clock, Start Menu launcher, PS/2 mouse cursor
- ✅ **M11: System Applications Suite** — Viking Prompt terminal shell, File Explorer VFS browser, VikingPad text editor, Task Manager process/memory monitor
- ✅ **M12: System Installer & Persistence** — MBR partition table generator, native persistent VikingFS block filesystem (Superblock `VIKI`), guided OS Installer Wizard deploying kernel and bootloader to hard disk

## Roadmap

See [Docs/TODO.md](Docs/TODO.md) for the detailed step-by-step checklist.

| # | Milestone | Status | Key Deliverable |
|---|---|---|---|
| 1 | Boot and initialization | ✅ Complete | Limine boot, serial diagnostics, memory probe |
| 2 | CPU Core Architecture | ✅ Complete | GDT, TSS, IST1, IDT, 32 CPU exceptions |
| 3 | Memory Management | ✅ Complete | PMM bitmap, 4-level VMM paging, dynamic heap |
| 4 | Hardware Interrupts & Timer | ✅ Complete | LAPIC, 100Hz timer, I/O APIC IRQ routing |
| 5 | Input Devices | ✅ Complete | PS/2 keyboard driver, scancode decoder |
| 6 | Graphics & Text Console | ✅ Complete | Linear framebuffer, 8x8 font, graphical console, GUI preview |
| 7 | Multitasking & Scheduler | ✅ Complete | Context switching, preemptive round-robin scheduler |
| 8 | Storage & Virtual Filesystem | ✅ Complete | Block device layer, VirtIO block, VFS, Ramdisk |
| 9 | User-Mode & System Calls | ✅ Complete | Ring 3 transition, `syscall` dispatcher, MSRs |
| 10 | Window Server & Desktop GUI | ✅ Complete | Full window compositor, movable windows, start menu, mouse cursor |
| 11 | System Applications | ✅ Complete | Viking Prompt, File Explorer, VikingPad, Task Manager |
| 12 | System Installer & Persistence | ✅ Complete | MBR partitioning, VikingFS filesystem, guided disk installer |

## Testing

### QEMU (Automated)

```bash
make test
```

### VMware Workstation

See [Docs/VMWARE_TESTING.md](Docs/VMWARE_TESTING.md) for step-by-step instructions.

## License

MIT License — see [LICENSE](LICENSE).
