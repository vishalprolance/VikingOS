# Roadmap: VikingOS

The development of VikingOS is divided into sequential milestones. Each milestone must be verified via automated tests or VM booting before proceeding.

## Completed Milestones

- [x] **Milestone 1: Boot & Initialization**
  - Limine bootloader protocol revision 3 integration.
  - 16550 UART COM1 serial diagnostic console (115200 8N1).
  - Physical memory map and HHDM enumeration.
  - Diagnostic panic handler and safe CPU halt loop.
- [x] **Milestone 2: CPU Core Architecture & Exception Handling**
  - Global Descriptor Table (GDT) with 64-bit Kernel/User Code & Data segments.
  - Task State Segment (TSS) with 16 KiB dedicated IST1 Double Fault stack.
  - 256-entry Interrupt Descriptor Table (IDT).
  - 32 x86-64 CPU exception handlers with formatted crash dumps.
  - In-kernel breakpoint trap verification (`int3`).
- [x] **Milestone 3: Memory Management Subsystem**
  - Physical Memory Manager (PMM) with bitmap frame allocation.
  - Virtual Memory Manager (VMM) with 4-level x86-64 paging (`map_page`, `unmap_page`, `translate`).
  - Kernel Dynamic Heap allocator with `alloc` crate integration (`Vec`, `String`, `Box`, `Arc`).
- [x] **Milestone 4: Hardware Interrupts & Timers**
  - Local APIC driver initialization and End of Interrupt (EOI) signaling.
  - APIC Timer configured to 100 Hz periodic mode.
  - I/O APIC driver and redirection table IRQ routing (IRQ 1 -> Vector 33).
  - Hardware interrupt dispatcher with atomic tick counter.
- [x] **Milestone 5: Input Devices**
  - PS/2 Keyboard driver reading status and data ports (0x60/0x64).
  - Scancode-to-ASCII translation.
  - Keyboard interrupt handler hooked to IDT vector 33.
- [x] **Milestone 6: Graphics & Framebuffer Console**
  - Linear framebuffer driver supporting 32-bit color rendering.
  - 8x8 ASCII bitmap font renderer (`draw_char`, `draw_string`).
  - Graphical text console with cursor management, newline handling, and scrolling.
  - 2D Canvas engine with shapes, rectangles, and colors.
- [x] **Milestone 7: Multitasking, Threading & Preemptive Scheduling**
  - Low-level AMD64 assembly context switching (`switch_context`).
  - Thread abstraction with state tracking (`Ready`, `Running`, `Blocked`, `Terminated`).
  - Preemptive Round-Robin scheduler driven by APIC timer ticks.
- [x] **Milestone 8: Storage Drivers & Virtual File System (VFS)**
  - `BlockDevice` trait and VirtIO block device driver.
  - Virtual File System core (`Inode`, `FileHandle`, `FileSystem`, `VfsError`).
  - In-memory read-only Ramdisk filesystem mounted at `/`.
- [x] **Milestone 9: User-Mode Execution & System Calls**
  - Ring 3 privilege transition stack frame setup (`iretq`).
  - CPU MSR configuration for fast `syscall`/`sysret` (`IA32_STAR`, `IA32_LSTAR`, `IA32_FMASK`).
  - System call dispatcher handling `Print`, `Exit`, `Malloc`, `GetStats`, `Kill`.
  - User process initialization and address space assignment.

---

## Active & Upcoming Milestones

### Phase 3: Desktop Shell & User Experience
- [ ] **Milestone 10: Window Server & Desktop GUI Compositor**
  - Full desktop window compositor with dirty-rectangle repainting.
  - Draggable windows with titlebars, close buttons, and border resizing.
  - Mouse pointer rendering and PS/2 mouse packet parsing.
  - Interactive Taskbar with active window buttons and clock.
  - Windows-inspired Start Menu with application launchers.
- [ ] **Milestone 11: System Applications**
  - Viking Prompt (in-kernel interactive command shell).
  - File Explorer browsing VFS and Ramdisk files.
  - VikingPad simple text viewer and editor.
  - Task Manager showing active threads, CPU ticks, and memory stats.

### Phase 4: Persistence & Release
- [ ] **Milestone 12: Installer & Persistent Boot**
  - Hard disk partitioning utility (MBR/GPT).
  - Limine bootloader disk installer (`limine bios-install`).
  - Installing VikingOS directly to a virtual disk or physical drive.
