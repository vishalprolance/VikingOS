# Architecture: VikingOS

VikingOS is an independent x86-64 operating system built from scratch in Rust. It follows a modular monolithic kernel architecture emphasizing memory safety, strong isolation, and eventual Windows application compatibility.

## System Layers

### 1. Boot Layer (Limine Protocol Revision 3)
- **Responsibility**: Boots the computer from BIOS or UEFI firmware and transfers control to the 64-bit kernel entry point `_start()`.
- **Delivered Context**: Higher-half paging, physical memory map, linear framebuffer info, and Higher-Half Direct Map (HHDM) base offset.

### 2. Core Architecture Layer (`kernel/src/arch/`)
- **GDT & TSS**: Sets up 64-bit Kernel Code (0x08), Kernel Data (0x10), User Data (0x1B), and User Code (0x23) segments. TSS holds a dedicated 16 KiB IST1 stack for infallible Double Fault exception recovery.
- **IDT & Exceptions**: 256-gate table. Assembly trampolines save all 15 general-purpose registers and route 32 CPU exceptions to formatted crash dump handlers. Breakpoint (`int3`) is configured as a user trap gate.
- **Port I/O**: Safe, inlined x86 `in` and `out` assembly wrappers in `arch/io.rs`.

### 3. Memory Subsystem (`kernel/src/memory/`)
- **Physical Memory Manager (PMM)**: Bitmap allocator tracking every 4 KiB physical page frame. Provides `allocate_frame()`, `allocate_frames()`, and `allocate_zeroed_frame()`.
- **Virtual Memory Manager (VMM)**: 4-level paging (PML4, PDPT, PD, PT) with fine-grained permissions (`PRESENT`, `WRITABLE`, `NO_EXECUTE`). Implements `map_page`, `unmap_page`, and TLB invalidation (`invlpg`).
- **Kernel Dynamic Heap**: Linked-list block allocator implementing `core::alloc::GlobalAlloc`, enabling the standard `alloc` crate (`Box`, `Vec`, `String`, `Arc`).

### 4. Interrupts & Hardware Drivers (`kernel/src/arch/`, `kernel/src/drivers/`)
- **Local APIC (LAPIC)**: Per-core interrupt controller managing EOI acknowledgment and driving the periodic 100 Hz scheduling timer.
- **I/O APIC**: Routes external hardware IRQs (e.g. Keyboard IRQ 1 -> Vector 33) to target CPU Local APICs.
- **PS/2 Keyboard**: Reads scancodes from port 0x60/0x64 and converts scancodes into ASCII characters.
- **PCI Bus**: Configuration space mechanism 1 scanning for peripheral devices.
- **Linear Framebuffer & Font**: Direct VRAM writing with 32-bit color, 8x8 ASCII font glyph renderer, and on-screen scrolling text console.
- **2D GUI Canvas**: Basic graphics primitives, window containers, and taskbar rendering.

### 5. Multitasking & Scheduling (`kernel/src/sync/`, `kernel/src/thread.rs`)
- **Context Switch**: Assembly routine (`switch_context`) that preserves callee-saved registers (RBP, RBX, R12-R15) and switches stack pointers.
- **Scheduler**: Preemptive Round-Robin scheduler invoked on every timer tick. Manages thread states: `Ready`, `Running`, `Blocked`, `Terminated`.

### 6. Storage & Virtual File System (`kernel/src/storage/`, `kernel/src/fs/`)
- **Block Device Layer**: Uniform `BlockDevice` trait for sector reading and writing.
- **VirtIO Block**: Paravirtualized block device driver for virtual machine storage.
- **Virtual File System (VFS)**: Abstractions for `Inode`, `FileHandle`, `FileSystem`, and directory traversal.
- **Ramdisk**: Read-only in-memory filesystem mounted at root (`/`).

### 7. User-Mode & System Calls (`kernel/src/process/`, `kernel/src/syscall/`)
- **Process Abstraction**: Tracks Process IDs, private CR3 page tables, kernel stacks, and user stack pointers.
- **Ring 3 Transition**: Prepares the AMD64 `iretq` stack frame (`SS`, `RSP`, `RFLAGS`, `CS`, `RIP`) to enter user space.
- **Fast System Calls**: Configures `IA32_STAR`, `IA32_LSTAR`, and `IA32_FMASK` MSRs for single-instruction `syscall`/`sysret` dispatch.

---

## Future Architectural Goals

### Desktop Window Compositor (Milestone 10)
- True 2D desktop compositor with dirty-rectangle rendering.
- Interactive window dragging, minimizing, maximizing, and closing.
- Taskbar with active window list, start button, and clock.

### Windows Application Compatibility Subsystem
- **PE32+ Loader**: Parsing Portable Executable headers, imports, exports, and sections.
- **Win32 API Layer**: Clean translation of Win32 system APIs to VikingOS kernel services.
