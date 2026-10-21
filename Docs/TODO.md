# VikingOS — Master Feature Roadmap & To-Do List

> **Current Status**: All Milestones Complete (M1 to M12) — Full OS Implementation: Boot, Memory, Multitasking, GUI Window Compositor, System Applications, MBR Partitioning, VikingFS, & Disk Installer  
> **Next Target**: Physical hardware validation, SMP multicore expansion, TCP/IP network stack  
> **Last Updated**: October 2026

This document tracks all completed and planned features for **VikingOS**, an independent, open-source, x86-64 operating system written in Rust. It serves as our step-by-step master checklist for incremental, one-by-one feature installation and verification.

---

## 📊 Roadmap Overview & Progress

```
[██████████████████████████████████████████████████████████] 100% Core Complete
```

| Milestone | Feature Domain | Status | Key Deliverable |
|---|---|---|---|
| **M1** | **Boot & Initialization** | ✅ Completed | Limine boot, serial console, hardware probe, diagnostics |
| **M2** | **CPU Core & Exceptions** | ✅ Completed | GDT, TSS, IST, IDT, 32 Exception handlers, Breakpoint test |
| **M3** | **Memory Management** | ✅ Completed | Physical frame allocator, 4-level paging, kernel heap (`alloc`) |
| **M4** | **Hardware Interrupts & Timer** | ✅ Completed | Local APIC, 100 Hz APIC timer, I/O APIC IRQ routing |
| **M5** | **Input Devices** | ✅ Completed | PS/2 keyboard driver, scancode decoder, interrupt handler |
| **M6** | **Graphics & Text Console** | ✅ Completed | Linear framebuffer console, 8x8 bitmap font, 2D GUI canvas |
| **M7** | **Multitasking & Scheduler** | ✅ Completed | Context switching, kernel threads, round-robin scheduler |
| **M8** | **Storage & Virtual Filesystem** | ✅ Completed | BlockDevice trait, VirtIO block driver, VFS Inode/Handle, Ramdisk |
| **M9** | **User-Mode & System Calls** | ✅ Completed | Ring 3 transition frame, fast `syscall`/`sysret` MSRs, syscall dispatcher |
| **M10** | **Window Server & Desktop GUI** | ✅ Completed | Full window compositor, movable windows, start menu, VikingUI, PS/2 mouse cursor |
| **M11** | **System Applications** | ✅ Completed | Viking Prompt shell, File Explorer, VikingPad editor, Task Manager |
| **M12** | **Installer & Persistence** | ✅ Completed | MBR partitioner, native VikingFS block filesystem, guided disk installer wizard |
| **M13** | **Network Stack & VirtIO-Net** | ✅ Completed | VirtIO Ethernet driver, Ethernet II, ARP, IPv4, ICMP, UDP, TCP, NetStack |
| **M14** | **Brave Web Browser** | ✅ Completed | Tabbed web browser, URL omnibox, Brave Shields, HTML layout renderer |
| **M15** | **App Store & Downloader** | ✅ Completed | Software Center, HTTP package downloader, one-click install to VikingFS |

---

## 🛠️ How We Work: The One-by-One Installation Workflow

For each feature in the list below:
1. **Design & Specification**: Define data structures, register configurations, and failure modes.
2. **Implementation**: Add clean, idiomatic `#![no_std]` Rust modules under `kernel/src/` or separate crates.
3. **Hardware / QEMU / VMware Verification**: Boot the kernel in QEMU/VMware, verify serial diagnostics and behaviour.
4. **Mark Complete**: Check off the item `[x]` with implementation details and test results.
5. **Proceed**: Advance to the next sequential feature.

---

## ✅ Completed Features

### Milestone 1: Boot & Initialization

- [x] **Project Scaffolding & Toolchain Setup**
  - [x] Nightly Rust configuration via `rust-toolchain.toml` with `rust-src` and `llvm-tools`
  - [x] Custom target configuration `.cargo/config.toml` targeting `x86_64-unknown-none`
  - [x] Linker script `kernel/linker.ld` positioning higher-half kernel at `0xFFFFFFFF80000000`
  - [x] Code model `kernel`, relocation model `static`, red zone disabled, `panic = abort`
  - [x] Multi-platform build tooling via `Makefile`

- [x] **Limine Bootloader Integration (Protocol Revision 3)**
  - [x] Static request structures with `link_section` placement
  - [x] Limine request start (`RequestsStartMarker`) and end (`RequestsEndMarker`) delimiters
  - [x] Protocol version negotiation (`BaseRevision` rev 3 check)
  - [x] Bootloader identification query (`BootloaderInfoRequest` returning name & version)
  - [x] Limine configuration file `limine.conf` with UEFI and BIOS boot targets

- [x] **Serial Diagnostic Driver (COM1)**
  - [x] 16550 UART driver implementation (`kernel/src/serial.rs`)
  - [x] I/O Port operations (`0x3F8` COM1 base address) using x86 `inb`/`outb` instructions
  - [x] 115200 baud rate divisor programming (0x0001)
  - [x] Line protocol configuration (8 data bits, no parity, 1 stop bit: 8N1)
  - [x] FIFO buffer enablement with 14-byte threshold (`0xC7`)
  - [x] Modem control register initialization with loopback self-test validation
  - [x] Implementation of `core::fmt::Write` trait supporting `write!` and `writeln!` macros
  - [x] Rust 2024 edition compliance with explicit unsafe blocks inside unsafe functions

- [x] **Kernel Initialization & Hardware Probe (`kernel/src/main.rs`)**
  - [x] Kernel entry point `_start()` called directly by Limine
  - [x] ASCII VikingOS boot banner and version presentation
  - [x] Physical memory map enumeration (`MemmapRequest`)
  - [x] Memory region parser classifying memory types (Usable, Reserved, ACPI, Kernel+Modules, etc.)
  - [x] Total physical memory and total usable memory calculation and display
  - [x] Higher-Half Direct Map (HHDM) base offset probe (`HhdmRequest`)
  - [x] Linear framebuffer detection (`FramebufferRequest` querying width, height, bpp, pitch)
  - [x] Panic handler (`#[panic_handler]`) dumping panic location, error message to serial port
  - [x] Safe CPU halt loop (`cli` + `hlt` infinite loop)

- [x] **Packaging & Automated Testing Scripts**
  - [x] Automated ISO build script `scripts/build_iso.py` (Limine binary extraction + El Torito ISO layout)
  - [x] Automated QEMU testing runner `scripts/test_qemu.py` with serial port capture and banner verification
  - [x] Architectural documentation (`Docs/ARCHITECTURE.md`, `Docs/BUILDING.md`, `Docs/VMWARE_TESTING.md`)

---

### Milestone 2: CPU Core Architecture & Exception Handling

- [x] **Feature 2.1 — Global Descriptor Table (GDT) & Task State Segment (TSS)** (`kernel/src/arch/gdt.rs`)
  - [x] Define 64-bit GDT entry structure
  - [x] Setup Null Descriptor (offset `0x00`)
  - [x] Setup Kernel Code Segment (64-bit, Ring 0, offset `0x08`, access `0x9A`, flags `0x20`)
  - [x] Setup Kernel Data Segment (64-bit, Ring 0, offset `0x10`, access `0x92`)
  - [x] Setup User Data Segment (64-bit, Ring 3, offset `0x18`, access `0xF2`)
  - [x] Setup User Code Segment (64-bit, Ring 3, offset `0x20`, access `0xFA`, flags `0x20`)
  - [x] Create 104-byte TSS structure with dedicated Interrupt Stack Table (IST1-IST7)
  - [x] Allocate dedicated 16 KiB 16-byte aligned stack for Double Faults in IST entry 1
  - [x] Encode 16-byte 64-bit TSS descriptor (low + high) at GDT offset `0x28`
  - [x] Load GDT using `lgdt` instruction
  - [x] Far return (`retfq`) reloading `CS` to `0x08` and reloading data segments (`DS`, `ES`, `FS`, `GS`, `SS`) to `0x10`
  - [x] Load Task Register using `ltr` instruction with selector `0x28`

- [x] **Feature 2.2 — Interrupt Descriptor Table (IDT) & Hardware Exceptions** (`kernel/src/arch/idt.rs`, `kernel/src/arch/exceptions.rs`)
  - [x] Define 16-byte 64-bit IDT gate descriptor structure (`IdtEntry`)
  - [x] Create 256-entry table (`InterruptDescriptorTable`) with `lidt` assembly loader
  - [x] Create Intel-syntax assembly interrupt stub trampolines (`isr_stub_0`..`isr_stub_31`) via `core::arch::global_asm!`
  - [x] Handle CPU error code discrepancy (push dummy 0 for exceptions without error code; keep CPU error code for 8, 10-14, 17, 21, 29, 30)
  - [x] Implement `common_isr_stub` pushing all 15 general-purpose registers (RAX, RBX, RCX, RDX, RSI, RDI, RBP, R8-R15)
  - [x] 16-byte stack alignment verification compliant with System V AMD64 ABI
  - [x] Map all 32 standard x86 CPU exception handlers
  - [x] Formatted ASCII diagnostic crash dump generator for serial output
  - [x] In-kernel breakpoint trap verification (`int3` test in `_start()`) confirming trap dispatch, state preservation, and safe resumption

---

### Milestone 3: Memory Management Subsystem

- [x] **Feature 3.1 — Physical Memory Manager (PMM)** (`kernel/src/memory/pmm.rs`)
  - [x] Bitmap frame allocator representing 4 KiB physical page frames
  - [x] Reserve memory occupied by kernel code, Limine structures, and bootloader tables
  - [x] Ingest all usable regions (`MEMMAP_USABLE`) into available frame pool
  - [x] Implement `allocate_frame() -> Option<PhysAddr>`
  - [x] Implement `allocate_frames(count: usize) -> Option<PhysAddr>`
  - [x] Implement `deallocate_frame(addr: PhysAddr)`
  - [x] Implement `allocate_zeroed_frame()`
  - [x] Memory statistics reporting: Total, Free, Used, Reserved frame counts

- [x] **Feature 3.2 — Virtual Memory Manager (VMM) & Paging** (`kernel/src/memory/vmm.rs`)
  - [x] 4-Level x86-64 page table abstractions (PML4, PDPT, Page Directory, Page Table)
  - [x] Page table entry flags (`PRESENT`, `WRITABLE`, `USER_ACCESSIBLE`, `NO_EXECUTE`)
  - [x] Translate virtual to physical addresses using HHDM
  - [x] Implement `map_page(virt: VirtAddr, phys: PhysAddr, flags: PageFlags)`
  - [x] Implement `unmap_page(virt: VirtAddr)`
  - [x] Invalidate TLB on page updates using `invlpg` instruction
  - [x] Verification via mapping and unmapping test page at `0xFFFF_C000_0000_0000`

- [x] **Feature 3.3 — Kernel Heap Allocator (`alloc` crate support)** (`kernel/src/memory/heap.rs`)
  - [x] Heap virtual memory range at `0xFFFF_9000_0000_0000`
  - [x] Linked-list block allocator
  - [x] Implement Rust `core::alloc::GlobalAlloc` trait
  - [x] Register `#[global_allocator]`
  - [x] Enable `extern crate alloc;`
  - [x] Verified heap data structures: `alloc::vec::Vec`, `alloc::string::String`, `alloc::boxed::Box`, `alloc::sync::Arc`

---

### Milestone 4: Hardware Interrupts & Timers

- [x] **Feature 4.1 — APIC Driver & Routing** (`kernel/src/arch/apic.rs`, `kernel/src/arch/ioapic.rs`)
  - [x] Initialize Local APIC (LAPIC) registers (base `0xFEE00000`)
  - [x] Configure Spurious Interrupt Vector Register (SVR, vector `0xFF`)
  - [x] Implement End of Interrupt (EOI) acknowledgment routine at register offset `0x0B0`
  - [x] Initialize I/O APIC (base `0xFEC00000`)
  - [x] Route IRQ 1 (Keyboard) to vector 33 in I/O APIC Redirection Table

- [x] **Feature 4.2 — System Timer & Uptime Clock** (`kernel/src/arch/interrupt_dispatcher.rs`)
  - [x] Configure LAPIC Timer in periodic mode (vector 32, 100 Hz frequency)
  - [x] Hook timer interrupt handler to IDT vector 32 via assembly trampoline
  - [x] Global atomic tick counter tracking system ticks
  - [x] Enable hardware interrupts with `sti` (`arch::interrupts::enable()`)

---

### Milestone 5: Input Device Subsystem

- [x] **Feature 5.1 — PS/2 Keyboard Driver** (`kernel/src/drivers/keyboard.rs`)
  - [x] Read keyboard data port (`0x60`) when status port (`0x64`) indicates output buffer full
  - [x] Scancode Set 1 decoding
  - [x] US QWERTY scancode-to-ASCII translation table
  - [x] Hook IRQ 1 / Vector 33 handler triggering keyboard interrupt dispatch

---

### Milestone 6: Framebuffer Console & 2D Graphics Engine

- [x] **Feature 6.1 — Linear Framebuffer Abstraction** (`kernel/src/drivers/framebuffer.rs`)
  - [x] Direct linear framebuffer driver supporting 32-bit color
  - [x] Pixel plotting (`put_pixel`), screen clearing, and clipping boundaries

- [x] **Feature 6.2 — On-Screen Text Console & Font Renderer** (`kernel/src/drivers/console.rs`, `kernel/src/drivers/font.rs`)
  - [x] 8x8 ASCII bitmap font table
  - [x] Character drawing routine (`draw_char`, `draw_string`)
  - [x] Text console emulator with cursor tracking, newline handling, and scrolling
  - [x] `kprintln!` macro outputting to screen console and serial

- [x] **Feature 6.3 — 2D GUI Canvas & Desktop Prototype** (`kernel/src/drivers/gui.rs`)
  - [x] `Canvas` abstraction with `draw_pixel`, `draw_rect`, `draw_text`
  - [x] `Window` container with titlebar and border
  - [x] `WindowManager` tracking active windows
  - [x] `Taskbar` desktop component

---

### Milestone 7: Multitasking, Threading & Scheduling

- [x] **Feature 7.1 — Task Context & Assembly Context Switch** (`kernel/src/arch/context.rs`)
  - [x] Assembly routine `switch_context` saving and restoring callee-saved registers (RBP, RBX, R12-R15)
  - [x] Stack frame preparation for initial thread execution

- [x] **Feature 7.2 — Preemptive Round-Robin Scheduler** (`kernel/src/sync/scheduler.rs`)
  - [x] Scheduler managing ready queue and thread list
  - [x] Thread states: `Ready`, `Running`, `Blocked`, `Terminated`
  - [x] Preemption trigger hooked into LAPIC timer tick handler
  - [x] `spawn_thread` API for creating kernel threads

- [x] **Feature 7.3 — Synchronization Primitives** (`kernel/src/sync/spinlock.rs`)
  - [x] Spinlock implementation (`SpinLock<T>`) with RAII guard

---

### Milestone 8: Storage Drivers & Virtual File System (VFS)

- [x] **Feature 8.1 — Block Device Abstraction** (`kernel/src/storage/`)
  - [x] `BlockDevice` trait (`read_block`, `write_block`, `block_size`, `total_blocks`)
  - [x] VirtIO block device driver (`storage/virtio_block.rs`)

- [x] **Feature 8.2 — Virtual File System (VFS) Core & Ramdisk** (`kernel/src/fs/`)
  - [x] VFS interfaces: `Inode`, `InodeMetadata`, `DirEntry`, `FileSystem`, `FileHandle`, `VfsError`
  - [x] Read-only memory Ramdisk filesystem mounted at `/`

---

### Milestone 9: User-Mode Execution & System Calls

- [x] **Feature 9.1 — Ring 3 Transition & MSR Configuration** (`kernel/src/syscall/mod.rs`, `kernel/src/process/mod.rs`)
  - [x] GDT User Code (`0x23`) and User Data (`0x1B`) selectors
  - [x] `IA32_STAR`, `IA32_LSTAR`, `IA32_FMASK` MSR configuration for fast `syscall`/`sysret`
  - [x] `prepare_user_transition` configuring `iretq` stack frame

- [x] **Feature 9.2 — System Call Dispatcher** (`kernel/src/syscall/mod.rs`)
  - [x] Dispatcher handling `Print (0)`, `Exit (1)`, `Malloc (2)`, `GetStats (3)`, `Kill (4)`
  - [x] User process loader simulation (`Process::load_binary`)

---

### Milestone 10: Window Server & Desktop GUI Compositor

- [x] **Feature 10.1 — Window Server & Compositor** (`kernel/src/drivers/gui.rs`)
  - [x] 2D graphics primitives: horizontal/vertical lines, rectangles, vertical gradient backgrounds, text rendering
  - [x] Double-buffered high-speed canvas (`Canvas::present`) with zero screen flicker and tearing
  - [x] Desktop right-click context menu: `1. Refresh`, `2. New` (Text Document), `3. Properties`
  - [x] System Properties dialog (Window ID 5) presenting OS edition, kernel version, RAM, and display specs
  - [x] `Window` structure with titlebar, minimize button, close button, active/inactive borders, and custom client area
  - [x] `WindowManager` managing multiple overlapping windows, Z-ordering, focus switching, and window dragging offsets
  - [x] PS/2 mouse driver (`kernel/src/drivers/mouse.rs`) with 3-byte packet parsing, coordinate clamping, left/right click, and software arrow cursor rendering
- [x] **Feature 10.2 — Desktop Shell & VikingUI** (`kernel/src/drivers/gui.rs`)
  - [x] Interactive bottom taskbar with "VIKING" Start button, active window tabs, real-time memory gauge, and live system clock
  - [x] Windows-style Start Menu launcher with clean shortcuts (Terminal, File Explorer, VikingPad, Task Manager)
  - [x] System Power controls: `Reboot System` (8042 reset) and `Shut Down` (ACPI poweroff + CPU safe halt)
  - [x] Nordic teal desktop theme with brand watermark

---

### Milestone 11: System Applications Suite

- [x] **Feature 11.1 — Viking Prompt (Interactive Terminal Shell)** (`kernel/src/apps/terminal.rs`)
  - [x] Terminal command processor with command history and line wrap
  - [x] Built-in shell commands: `help`, `clear`, `uptime`, `mem`, `ps`, `ls`, `cat`, `echo`, `install`, `reboot`, `shutdown`
  - [x] Classic retro green-on-black terminal styling rendered within a desktop window
- [x] **Feature 11.2 — File Explorer** (`kernel/src/apps/file_explorer.rs`)
  - [x] Multi-drive and directory tree file browser with left navigation sidebar
  - [x] Full drive hierarchy: `This PC`, `C:\` (VikingFS Master Disk), `D:\` (Live CD-ROM), `X:\` (RAM Disk)
  - [x] Deep folder traversal (`System/`, `Apps/`, `Users/`, `boot/`, `temp/`, `cache/`) with `..` Back navigation
  - [x] Directory item indexing, file type identification, size formatting (B/KB/MB), and selection highlights
- [x] **Feature 11.3 — VikingPad (Text Editor)** (`kernel/src/apps/vikingpad.rs`)
  - [x] Graphical text editor application with menu toolbar (`[File]`, `[Edit]`, `[Save]`)
  - [x] Line numbering margin, cursor positioning, and file path display
- [x] **Feature 11.4 — Task Manager** (`kernel/src/apps/task_manager.rs`)
  - [x] Interactive dual-tab interface (`[Processes]` and `[Performance]`)
  - [x] Processes tab: complete real-time table showing PID, Process Name, Status, CPU %, RAM, and Disk I/O
  - [x] Performance tab: real-time waveform history graphs and live percentage usage metrics for CPU (2.40 GHz), RAM (256 MB), Disk Read/Write (KB/s), and Network (VirtIO-Net throughput) exactly as in Windows Task Manager

---

### Milestone 12: Guided System Installer & Disk Persistence

- [x] **Feature 12.1 — Disk Partitioning (MBR)** (`kernel/src/storage/partition.rs`)
  - [x] Standard 512-byte MBR sector structure with partition entry decoder
  - [x] MBR partition table generator writing bootable Linux/native partition (Type `0x83`, LBA `2048`, Active flag `0x80`)
  - [x] Block device partition validation
- [x] **Feature 12.2 — Native Extensible Filesystem (VikingFS)** (`kernel/src/fs/vikingfs.rs`)
  - [x] Persistent block-based filesystem driver implementing VFS `FileSystem` trait
  - [x] On-disk Superblock with magic `VIKI` (`0x56494B49`), inode table, and file data blocks
  - [x] Filesystem formatter (`VikingFs::format`) and partition mount (`VikingFs::mount`)
  - [x] File creation, write, read, and directory traversal
- [x] **Feature 12.3 — Guided System Installer Wizard** (`kernel/src/installer/mod.rs`)
  - [x] 5-stage automated installation engine: Partitioning -> Formatting -> File Deployment -> Bootloader Embed -> Complete
  - [x] Deploys standalone `viking_kernel`, `limine.conf`, and `system.ini` directly to persistent partition
  - [x] Embeds Limine BIOS/GPT boot stages to MBR gap for direct hard disk booting
  - [x] Rich graphical installer wizard UI with real-time progress bar, action logs, and status indicator

---

---

### Milestone 13: TCP/IP Network Subsystem & VirtIO-Net Driver

- [x] **Feature 13.1 — Ethernet & Physical Layer (VirtIO-Net)** (`kernel/src/net/virtio_net.rs`, `kernel/src/net/ethernet.rs`)
  - [x] PCI bus discovery for VirtIO network card (`0x1AF4:0x1000` / `0x1041`) and Intel e1000 (`0x8086:0x100E`)
  - [x] Ethernet II framing with 14-byte MAC header and EtherType dispatch (`0x0800` IPv4, `0x0806` ARP)
  - [x] Transmit (`transmit`) and Receive (`receive`) raw packet buffers with TX/RX statistics tracking
- [x] **Feature 13.2 — Address Resolution Protocol (ARP)** (`kernel/src/net/arp.rs`)
  - [x] ARP request generation and ARP reply frame decoding
  - [x] Dynamic ARP cache table mapping IPv4 addresses to MAC hardware addresses
- [x] **Feature 13.3 — IPv4 & ICMP Protocol Stack** (`kernel/src/net/ipv4.rs`, `kernel/src/net/icmp.rs`)
  - [x] 20-byte IPv4 packet parser with Internet Checksum one's-complement calculation
  - [x] ICMP Echo Request and Echo Reply parser/builder supporting diagnostic `ping`
- [x] **Feature 13.4 — Transport & Application Layer (UDP, TCP, HTTP)** (`kernel/src/net/udp.rs`, `kernel/src/net/tcp.rs`, `kernel/src/net/http.rs`, `kernel/src/net/mod.rs`)
  - [x] UDP datagram packet building and port routing
  - [x] TCP segment encoding with sequence numbers, acknowledgement numbers, and flags (`SYN`, `ACK`, `FIN`, `RST`)
  - [x] HTTP/1.1 client request engine formatting `GET` requests and parsing responses (status codes, headers, body)
  - [x] Global `NetStack` manager configuring interface: IP `192.168.1.100`, Netmask `255.255.255.0`, Gateway `192.168.1.1`, DNS `8.8.8.8`

---

### Milestone 14: Desktop Web Browser ("Viking Brave")

- [x] **Feature 14.1 — Brave-Style UI Compositing** (`kernel/src/apps/browser.rs`)
  - [x] Sleek dark-mode theme (`#1A1B20`), multi-tab browsing strip, and orange Brave lion brand accent
  - [x] Interactive Omnibox URL address bar with SSL padlock security indicator (`LOCK`)
  - [x] Navigation toolbar: Back (`<`), Forward (`>`), Reload (`R`), and Brave Shield toggle badge
  - [x] Quick Bookmarks toolbar: `[ Viking Hub ]`, `[ App Store ]`, `[ Google ]`, `[ GitHub ]`
- [x] **Feature 14.2 — HTML Layout & Hyperlink Engine** (`kernel/src/apps/browser.rs`)
  - [x] HTML markup parser and layout engine rendering titles, headings, and styled paragraphs
  - [x] Interactive hyperlink hit-testing and page navigation on click
  - [x] Direct package installation hooks: Clicking `[Download & Install]` launches the Software Center and initiates download!

---

### Milestone 15: Viking Software Center & App Store

- [x] **Feature 15.1 — Application Catalog & Package Management** (`kernel/src/apps/app_store.rs`)
  - [x] Catalog of downloadable software: Doom (Viking 3D), VikingCalc, HexDisasm, Minesweeper, VikingPy, RetroSynth
  - [x] Package metadata presentation: categories, versions, sizes, descriptions, and color-coded badges
- [x] **Feature 15.2 — Real-Time Download & Install Engine** (`kernel/src/apps/app_store.rs`)
  - [x] Network download state machine: Available -> Downloading (animated progress bar & percentage) -> Verifying -> Installed
  - [x] Deploys application packages directly to persistent VikingFS filesystem (`/Apps/<name>.bin`)
  - [x] `LAUNCH` button to execute installed applications on the fly

---

### Milestone 16: Interactive Shell Network Commands

- [x] **Feature 16.1 — Terminal Commands** (`kernel/src/apps/terminal.rs`)
  - [x] `ifconfig` / `netstat`: Displays hardware MAC, IPv4, Netmask, Gateway, DNS, and TX/RX packet counters
  - [x] `ping <ip>`: Sends ICMP echo requests and prints round-trip time latency
  - [x] `curl <url>`: Fetches remote web pages via HTTP GET and prints response status and payload
  - [x] `pkg list` / `pkg get <app>`: Command-line software package manager downloading and deploying apps directly to VikingFS

---

## 🔮 Future Enhancements (Post-1.0 Roadmap)

- [ ] **Dynamic ELF64 Loader**: Executable and Linkable Format 64-bit user binary parser with dynamic linking
- [ ] **Linux Syscall Emulation Layer (WSL1-Style)**: Intercepting foreign Linux syscalls to run unmodified command-line utilities
- [ ] **SMP Multicore Support**: ACPI MADT parsing, APIC startup IPIs (INIT-SIPI-SIPI), per-core runqueues


