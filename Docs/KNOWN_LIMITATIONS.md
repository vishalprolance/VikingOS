# Known Limitations: VikingOS

This document lists the current gaps, planned enhancements, and technical constraints of VikingOS as of Milestone 9 completion.

## Subsystems Implemented & Working
The following core subsystems are functional and operational in the kernel:
- ✅ **Bootloader Protocol**: Limine Protocol Revision 3 (UEFI + BIOS)
- ✅ **CPU Architecture**: GDT, TSS (IST1 Double Fault), 256-entry IDT, 32 CPU exceptions
- ✅ **Memory Management**: PMM (Bitmap allocator), VMM (4-level paging), Dynamic Heap (`alloc` crate)
- ✅ **Interrupts & Timers**: Local APIC, 100 Hz timer ticks, I/O APIC IRQ routing
- ✅ **Input Devices**: PS/2 Keyboard scancode decoder
- ✅ **Graphics & Display**: Linear Framebuffer, 8x8 font, on-screen console, 2D GUI canvas & desktop preview
- ✅ **Multitasking**: AMD64 context switching, preemptive round-robin scheduler, thread state tracking
- ✅ **Storage & VFS**: `BlockDevice` trait, VirtIO block driver, VFS abstractions, native persistent VikingFS
- ✅ **User-Mode**: Ring 3 transition frame preparation, fast `syscall`/`sysret` MSR configuration & dispatcher
- ✅ **Desktop GUI Compositor**: Multi-window manager, Z-ordering, taskbar, start menu, double-buffered canvas, PS/2 mouse cursor
- ✅ **Networking Subsystem**: VirtIO-Net PCI driver, Ethernet II, ARP, IPv4, ICMP (ping), UDP, TCP, HTTP/1.1 client
- ✅ **Desktop Web Browser**: Brave-style tabbed navigator, URL Omnibox with SSL lock, privacy shields, HTML rendering
- ✅ **Software Center & App Store**: One-click HTTP package downloader, checksum verification, and persistent VikingFS installer

---

## Technical Constraints & Reverse Engineering Analysis: Running Existing Chrome / Brave / External Binaries

### 1. Why Existing Unmodified Chrome / Brave Binaries Cannot Run Directly on a Custom OS Kernel
Existing browsers like Google Chrome or Brave are built on top of **Chromium** (over 35 million lines of C++ code). A binary reverse engineering inspection reveals that existing Chrome binaries (`chrome` on Linux, `chrome.exe` on Windows) are not standalone programs; they are host-system orchestrators with extreme dependency trees:
- **Foreign ABI & Dynamic Linking**: Linux Chrome is an ELF64 executable dynamically linked against `glibc`, `libstdc++`, `libX11`, `libwayland`, `libasound`, `libgtk-3`, `libnss3`, and dozens of system shared objects.
- **Syscall Interception Requirements**: Chromium issues over 150+ Linux syscalls (`sys_clone` with user namespaces, `sys_futex`, `sys_epoll_ctl`, `sys_seccomp` sandboxing, `sys_mmap`, `sys_socket`, `sys_connect`). On VikingOS, foreign syscall IDs do not match unless a kernel-level Linux ABI translation layer (like FreeBSD's Linuxulator or Microsoft's WSL1) is implemented.
- **Display & Graphics Stack**: Chrome delegates all hardware rendering and window management to an X11 server or Wayland compositor using shared memory buffers (`wl_shm`) or GPU hardware textures (DRM / KMS / Mesa).
- **Memory Footprint**: Modern Chromium requires a minimum of 2 GB of RAM just to initialize its multi-process sandbox architecture.

### 2. Roadmap to Foreign Binary Execution on VikingOS
To achieve binary execution of external binaries on VikingOS, the following architectural milestones are planned:
1. **Dynamic ELF64 Loader**: Map standard Linux ELF binaries into user-space virtual memory and resolve PLT/GOT relocations.
2. **Linux Syscall Emulation Layer (WSL1-Style)**: Intercept Linux system call numbers inside the `syscall` MSR dispatcher and translate them into native VikingOS kernel primitives.
3. **Lightweight Browser Engines**: Porting lighter open-source layout engines (such as NetSurf, Ladybird, or lightweight WebAssembly runtimes) that do not require multi-gigabyte browser stacks.

