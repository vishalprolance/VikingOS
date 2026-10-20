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
- ✅ **Storage & VFS**: `BlockDevice` trait, VirtIO block driver, VFS abstractions, in-memory Ramdisk
- ✅ **User-Mode**: Ring 3 transition frame preparation, fast `syscall`/`sysret` MSR configuration & dispatcher

---

## Current Limitations & Next Steps

### 1. GUI & Compositor (Milestone 10)
- **Static Window Layout**: Windows and Taskbar are currently drawn once on boot; dynamic drag-and-drop window movement and mouse event dispatching are being added in Milestone 10.
- **PS/2 Mouse Driver**: Mouse packet parser is pending to support hardware cursor movement.

### 2. User-Mode Application Execution
- **Embedded vs ELF Binary**: User processes are currently simulated via embedded binary structures; a full ELF64/PE parser for disk-loaded executables is the next step.
- **Preemptive Return from Ring 3**: Timer interrupts during user mode will save user context and return via `sysretq`/`iretq`.

### 3. Storage & Filesystems
- **Read-Only Ramdisk**: The currently mounted filesystem is a read-only ramdisk. Read/write support on partitionable block media (e.g. FAT32 or VikingFS) is scheduled for Milestone 12.
- **AHCI / NVMe**: Hardware block access is currently focused on VirtIO; native SATA/AHCI and NVMe controllers are future drivers.

### 4. Networking
- **No Network Stack**: No Ethernet controller drivers (e.g. e1000, virtio-net) or TCP/IP stack implemented yet.

### 5. Windows Compatibility Subsystem
- **No PE32+ Loader**: Executables currently use internal binary format; PE loader and Win32 emulation will follow in Phase 3.
