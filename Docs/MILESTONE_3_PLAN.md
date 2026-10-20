# VikingOS — Milestone 3: Memory Management Subsystem Plan

This document outlines the architectural plan and technical specifications for implementing **Milestone 3: Memory Management Subsystem** in VikingOS.

---

## 1. Architectural Overview

Memory management in VikingOS is organized into three sequential, interdependent layers:

```
+-------------------------------------------------------+
|                Layer 3: Kernel Heap                   |
|   `alloc` crate support: Box, Vec, String, Arc, etc.   |
|     Free-List / Linked-List Allocator (GlobalAlloc)    |
+-------------------------------------------------------+
                           |
                           v
+-------------------------------------------------------+
|          Layer 2: Virtual Memory Manager (VMM)         |
|         4-Level x86-64 Paging (PML4, PDPT, PD, PT)    |
|   HHDM-based page walking, map_page(), unmap_page()   |
+-------------------------------------------------------+
                           |
                           v
+-------------------------------------------------------+
|         Layer 1: Physical Memory Manager (PMM)         |
|             4 KiB Bitmap Frame Allocator              |
|   Limine memory map ingestion, allocate_frame(),      |
|             deallocate_frame(), stats                 |
+-------------------------------------------------------+
```

---

## 2. Component Design & Specifications

### 2.1 Synchronization Primitive (`kernel/src/sync/spinlock.rs`)
- Atomic spinlock using `core::sync::atomic::AtomicBool` with `Acquire` / `Release` memory ordering.
- RAII `SpinLockGuard<'a, T>` providing `Deref` and `DerefMut`.
- Used to protect:
  1. Serial console logging (`kprint!` / `kprintln!`)
  2. Physical Frame Allocator (`PMM`)
  3. Page Tables (`VMM`)
  4. Global Heap Allocator (`GlobalAlloc`)

### 2.2 Address Types & HHDM (`kernel/src/memory/mod.rs`)
- `PhysAddr(pub u64)`: Strongly-typed physical addresses with 4 KiB alignment checks and arithmetic.
- `VirtAddr(pub u64)`: Strongly-typed virtual addresses with page table index extractions (`pml4_index()`, `pdpt_index()`, `pd_index()`, `pt_index()`, `page_offset()`).
- Higher-Half Direct Map (HHDM) global offset management:
  - `phys_to_virt(PhysAddr) -> VirtAddr`
  - `virt_to_phys(VirtAddr) -> PhysAddr`

### 2.3 Physical Memory Manager (PMM) (`kernel/src/memory/pmm.rs`)
- **Page Frame Size**: 4096 bytes (4 KiB).
- **Allocator**: Fast Bitmap Allocator.
  - Sized based on the highest physical address discovered in the Limine memory map.
  - Dynamically located in the first sufficiently large usable physical memory region.
  - Accessed directly via HHDM virtual addresses.
- **Initialization Workflow**:
  1. Mark entire bitmap as used (`0xFF`).
  2. Parse Limine `MemmapResponse` entries and mark all `MEMMAP_USABLE` frames as free (`0x00`).
  3. Reserve frames occupied by the bitmap itself.
  4. Reserve frame 0 (`0x0000_0000 .. 0x0000_1000`) to prevent NULL pointer physical allocations.
- **API**:
  - `allocate_frame() -> Option<PhysAddr>`
  - `allocate_frames(count: usize) -> Option<PhysAddr>`
  - `deallocate_frame(addr: PhysAddr)`
  - `deallocate_frames(addr: PhysAddr, count: usize)`
  - `allocate_zeroed_frame() -> Option<PhysAddr>`
  - Statistics: Total, Free, Used frame counts and byte calculations.

### 2.4 Virtual Memory Manager (VMM) (`kernel/src/memory/vmm.rs`)
- **Paging Model**: x86-64 4-Level Paging (PML4, PDPT, PD, PT).
- **Entry Flags (`PageFlags`)**:
  - `PRESENT` (`1 << 0`)
  - `WRITABLE` (`1 << 1`)
  - `USER_ACCESSIBLE` (`1 << 2`)
  - `WRITE_THROUGH` (`1 << 3`)
  - `CACHE_DISABLE` (`1 << 4`)
  - `ACCESSED` (`1 << 5`)
  - `DIRTY` (`1 << 6`)
  - `HUGE_PAGE` (`1 << 7`)
  - `GLOBAL` (`1 << 8`)
  - `NO_EXECUTE` (`1 << 63`)
- **Active Page Table**: Read current PML4 from `CR3`.
- **Page Table Walking & Mapping**:
  - `map_page(virt, phys, flags)`: Walks levels 4 -> 3 -> 2 -> 1. If intermediate tables are missing, allocates zeroed 4 KiB frames from PMM and links them.
  - `unmap_page(virt)`: Clears leaf PT entry and invalidates TLB.
  - `translate(virt)`: Resolves virtual address to physical address.
  - `change_page_flags(virt, flags)`: Modifies protection bits.
  - TLB Invalidation: Uses `invlpg` assembly instruction.

### 2.5 Kernel Heap Allocator (`kernel/src/memory/heap.rs`)
- **Virtual Address Space**: Dedicated range `0xFFFF_A000_0000_0000` to `0xFFFF_A000_4000_0000` (1 GiB virtual heap arena).
- **Initial Heap**: Pre-maps 2 MiB (512 frames) backed by PMM and VMM.
- **Allocator Algorithm**: Robust Free-List Allocator.
  - Each block has a `BlockHeader` tracking block size, allocation status (`is_free`), and intrusive linked list pointers.
  - Block splitting when remaining free space is sufficient for a new block.
  - Block coalescing on `dealloc()` to merge adjacent free blocks and eliminate fragmentation.
  - Dynamic heap growth: If heap runs out of memory, allocates additional pages via VMM and PMM automatically.
- **Rust Integration**:
  - Implements `core::alloc::GlobalAlloc` for `LockedHeap`.
  - Decorated with `#[global_allocator]`.
  - Enables `extern crate alloc;`.
- **Validation**:
  - Tests dynamic heap collections: `Box<T>`, `Vec<T>`, `String`, `format!`, and `Arc<T>`.

---

## 3. Implementation Steps

1. `kernel/src/sync/mod.rs` & `kernel/src/sync/spinlock.rs`
2. Update `kernel/src/serial.rs` with safe `kprint!` and `kprintln!` macros
3. `kernel/src/memory/mod.rs` (Address types & HHDM)
4. `kernel/src/memory/pmm.rs` (Bitmap physical frame allocator)
5. `kernel/src/memory/vmm.rs` (4-level paging and page table walker)
6. `kernel/src/memory/heap.rs` (Kernel heap allocator and `GlobalAlloc`)
7. `kernel/src/main.rs` integration, memory diagnostics, and heap test suite
8. Update `Docs/TODO.md` to reflect Milestone 3 completion
9. Verify compilation with `cargo build --release`
