# Project Status: VikingOS

This document tracks the implementation status of VikingOS features and the evidence required to mark them complete.

## Implementation Matrix

| Feature | Status | Evidence | Notes |
| :--- | :---: | :--- | :--- |
| **Boot & Initialization (M1)** | ✅ Complete | ISO boots in QEMU/VMware; Limine protocol requests fulfilled. | Verified in `main.rs` |
| **Serial Diagnostics (M1)** | ✅ Complete | COM1 UART output verified at 115200 baud 8N1. | Verified in `serial.rs` |
| **CPU Core Architecture (M2)** | ✅ Complete | GDT loaded; TSS active with IST1 Double Fault stack (16 KiB). | Verified in `arch/gdt.rs` |
| **Exception Handling (M2)** | ✅ Complete | IDT loaded (256 vectors); `int3` trap verified; 32 exception handlers. | Verified in `arch/exceptions.rs` |
| **Physical Memory (PMM) (M3)** | ✅ Complete | Bitmap allocator active; frame allocation, zeroing, deallocation verified. | Verified in `memory/pmm.rs` |
| **Virtual Memory (VMM) (M3)** | ✅ Complete | 4-level paging active; `map_page`, `unmap_page`, `translate` verified. | Verified in `memory/vmm.rs` |
| **Kernel Heap Allocator (M3)** | ✅ Complete | `GlobalAlloc` implemented; `Box`, `Vec`, `String`, `Arc` operational. | Verified in `memory/heap.rs` |
| **Synchronization (M3/M7)** | ✅ Complete | `SpinLock<T>` implemented with RAII guards. | Verified in `sync/spinlock.rs` |
| **Hardware Interrupts (M4)** | ✅ Complete | Local APIC initialized; I/O APIC routes IRQ 1 -> Vector 33; EOI working. | Verified in `arch/apic.rs`, `arch/ioapic.rs` |
| **System Timer (M4)** | ✅ Complete | LAPIC Timer set to 100 Hz periodic mode; atomic tick counter active. | Verified in `arch/interrupt_dispatcher.rs` |
| **Input Devices (M5)** | ✅ Complete | PS/2 Keyboard driver implemented; scancode-to-ASCII decoder working. | Verified in `drivers/keyboard.rs` |
| **Graphics & Text Console (M6)** | ✅ Complete | Linear framebuffer, 8x8 font rendering, scrolling text console. | Verified in `drivers/console.rs` |
| **GUI Desktop Engine (M6/M10)** | ✅ Complete | 2D Canvas, Window Manager, Windows, Taskbar preview rendered. | Verified in `drivers/gui.rs` |
| **Preemptive Scheduler (M7)** | ✅ Complete | AMD64 context switching (`switch_context`), Round-Robin scheduler. | Verified in `sync/scheduler.rs` |
| **Kernel Threading (M7)** | ✅ Complete | `Thread` abstraction, multiple threads created and preempted via timer. | Verified in `thread.rs` |
| **Storage & VFS (M8)** | ✅ Complete | `BlockDevice` trait, VirtIO block driver, VFS Inode/Handle, Ramdisk mounted. | Verified in `storage/`, `fs/` |
| **User-Mode & Syscalls (M9)** | ✅ Complete | Ring 3 transition frame (`iretq`), MSRs configured (`STAR`, `LSTAR`, `FMASK`), syscall dispatcher. | Verified in `process/`, `syscall/` |
| **Window Server & Compositor (M10)** | ✅ Complete | Window compositor, multi-window Z-ordering, dragging, Taskbar with RAM/clock, Start Menu launcher, PS/2 mouse cursor. | Verified in `drivers/gui.rs`, `drivers/mouse.rs` |
| **System Applications (M11)** | ✅ Complete | Viking Prompt shell, File Explorer VFS browser, VikingPad text editor, Task Manager process/memory monitor. | Verified in `apps/` |
| **Installer & Persistence (M12)** | ✅ Complete | MBR partition table generator, VikingFS block filesystem (superblock `VIKI`), guided Installer Wizard engine deploying kernel and bootloader. | Verified in `storage/partition.rs`, `fs/vikingfs.rs`, `installer/` |

## Verified Baselines

- **Target**: `x86_64-unknown-none`
- **Bootloader**: Limine Protocol Revision 3 (BIOS + UEFI)
- **Memory Layout**: Higher-half kernel (`0xFFFFFFFF80000000`), HHDM (`0xFFFF800000000000`), Heap (`0xFFFF900000000000`)
- **Toolchain**: Rust Nightly (Edition 2024 compliance)
- **Build Output**: `build/VikingOS.iso` (El Torito hybrid ISO)
