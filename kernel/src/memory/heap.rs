// VikingOS Kernel — Heap Allocator
//
// Dynamic kernel memory allocator implementing Rust's `core::alloc::GlobalAlloc`
// trait. Enables standard library collections (`alloc::boxed::Box`, `alloc::vec::Vec`,
// `alloc::string::String`, `alloc::sync::Arc`) via `#![no_std]` and `extern crate alloc`.
//
// Implements a free-list block allocator with block splitting, alignment padding,
// adjacent block coalescing, and automatic heap growth.

#![allow(dead_code)]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use crate::kprintln;
use crate::memory::{pmm, vmm, PageFlags, VirtAddr};
use crate::sync::SpinLock;

/// Virtual base address for the kernel heap arena.
pub const HEAP_START: u64 = 0xFFFF_A000_0000_0000;

/// Initial size of the kernel heap (2 MiB = 512 frames).
pub const HEAP_INITIAL_SIZE: usize = 2 * 1024 * 1024;

/// Maximum size of the kernel heap (128 MiB).
pub const HEAP_MAX_SIZE: usize = 128 * 1024 * 1024;

/// Magic identifier to validate block headers ("VIKG").
const BLOCK_MAGIC: u32 = 0x5649_4B47;

/// Minimum block data payload size after splitting.
const MIN_SPLIT_PAYLOAD: usize = 32;

// ── Block Header ─────────────────────────────────────────────────────

/// Metadata header preceding each allocated or free heap chunk.
#[repr(C, align(16))]
pub struct BlockHeader {
    pub magic: u32,
    pub is_free: bool,
    pub size: usize,
    pub padding: usize,
    pub next: Option<NonNull<BlockHeader>>,
    pub prev: Option<NonNull<BlockHeader>>,
}

const HEADER_SIZE: usize = core::mem::size_of::<BlockHeader>();

impl BlockHeader {
    /// Return the raw data pointer directly following this header.
    #[inline(always)]
    pub fn data_ptr(&self) -> *mut u8 {
        let base = self as *const Self as usize;
        (base + HEADER_SIZE + self.padding) as *mut u8
    }

    /// Total memory footprint of this block (header + padding + data size).
    #[inline(always)]
    pub fn total_size(&self) -> usize {
        HEADER_SIZE + self.padding + self.size
    }
}

// ── Free-List Heap Allocator ─────────────────────────────────────────

pub struct HeapAllocator {
    head: Option<NonNull<BlockHeader>>,
    current_size: usize,
    allocated_bytes: usize,
}

unsafe impl Send for HeapAllocator {}

impl HeapAllocator {
    /// Create a new heap allocator managing an initialized memory region.
    pub unsafe fn new(start: VirtAddr, initial_size: usize) -> Self {
        let head_ptr = start.as_mut_ptr::<BlockHeader>();
        unsafe {
            head_ptr.write(BlockHeader {
                magic: BLOCK_MAGIC,
                is_free: true,
                size: initial_size - HEADER_SIZE,
                padding: 0,
                next: None,
                prev: None,
            });
        }

        Self {
            head: NonNull::new(head_ptr),
            current_size: initial_size,
            allocated_bytes: 0,
        }
    }

    /// Allocate memory satisfying `layout`.
    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let align = layout.align().max(16);
        let size = (layout.size().max(1) + 15) & !15; // 16-byte align size

        // First attempt: find a fitting block in existing heap
        if let Some(ptr) = self.find_and_allocate(size, align) {
            return ptr;
        }

        // Second attempt: grow heap and retry
        let grow_needed = (size + HEADER_SIZE + 4095) / 4096 * 4096;
        let grow_size = grow_needed.max(256 * 1024); // Grow at least 256 KiB
        if self.grow(grow_size) {
            if let Some(ptr) = self.find_and_allocate(size, align) {
                return ptr;
            }
        }

        core::ptr::null_mut()
    }

    /// Scan the free list for a block that can accommodate `size` bytes with `align`.
    fn find_and_allocate(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let mut curr = self.head;

        while let Some(mut block_ptr) = curr {
            let block = unsafe { block_ptr.as_mut() };
            if block.magic != BLOCK_MAGIC {
                kprintln!("[FATAL] Heap corruption detected: invalid block magic {:#x}", block.magic);
                return None;
            }

            if block.is_free {
                let header_addr = block as *mut BlockHeader as usize;
                let raw_data_addr = header_addr + HEADER_SIZE;
                let aligned_data_addr = (raw_data_addr + align - 1) & !(align - 1);
                let padding = aligned_data_addr - raw_data_addr;

                if block.size >= size + padding {
                    let excess = block.size - (size + padding);

                    // Split block if excess is large enough
                    if excess >= HEADER_SIZE + MIN_SPLIT_PAYLOAD {
                        let new_header_addr = aligned_data_addr + size;
                        let new_block_ptr = new_header_addr as *mut BlockHeader;

                        unsafe {
                            new_block_ptr.write(BlockHeader {
                                magic: BLOCK_MAGIC,
                                is_free: true,
                                size: excess - HEADER_SIZE,
                                padding: 0,
                                next: block.next,
                                prev: Some(block_ptr),
                            });

                            if let Some(mut next_ptr) = block.next {
                                next_ptr.as_mut().prev = NonNull::new(new_block_ptr);
                            }
                            block.next = NonNull::new(new_block_ptr);
                        }

                        block.size = size;
                    }

                    block.padding = padding;
                    block.is_free = false;
                    self.allocated_bytes += block.size;

                    // If padding > 0, store header address right before aligned_data_addr for dealloc retrieval
                    if padding >= 8 {
                        unsafe {
                            let backpointer = (aligned_data_addr - 8) as *mut usize;
                            *backpointer = header_addr;
                        }
                    }

                    return Some(aligned_data_addr as *mut u8);
                }
            }

            curr = block.next;
        }

        None
    }

    /// Deallocate a previously allocated heap pointer.
    pub fn dealloc(&mut self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }

        let ptr_addr = ptr as usize;

        // Retrieve the BlockHeader corresponding to this allocation
        let header_ptr = unsafe {
            // First check if padding was 0: header is at (ptr - HEADER_SIZE)
            let direct_header = (ptr_addr - HEADER_SIZE) as *mut BlockHeader;
            if (*direct_header).magic == BLOCK_MAGIC && (*direct_header).padding == 0 {
                direct_header
            } else {
                // Otherwise retrieve stored backpointer at (ptr - 8)
                let stored_header = *((ptr_addr - 8) as *const usize) as *mut BlockHeader;
                if (*stored_header).magic == BLOCK_MAGIC {
                    stored_header
                } else {
                    kprintln!("[WARN ] Heap dealloc: invalid block header pointer");
                    return;
                }
            }
        };

        let block = unsafe { &mut *header_ptr };
        if block.is_free {
            kprintln!("[WARN ] Heap dealloc: double free detected at {:#x}", ptr_addr);
            return;
        }

        block.is_free = true;
        self.allocated_bytes = self.allocated_bytes.saturating_sub(block.size);

        // Coalesce with next block if next is free
        if let Some(mut next_ptr) = block.next {
            let next_block = unsafe { next_ptr.as_mut() };
            if next_block.is_free {
                block.size += next_block.total_size();
                block.next = next_block.next;
                if let Some(mut after_ptr) = block.next {
                    unsafe {
                        after_ptr.as_mut().prev = NonNull::new(header_ptr);
                    }
                }
            }
        }

        // Coalesce with prev block if prev is free
        if let Some(mut prev_ptr) = block.prev {
            let prev_block = unsafe { prev_ptr.as_mut() };
            if prev_block.is_free {
                prev_block.size += block.total_size();
                prev_block.next = block.next;
                if let Some(mut next_ptr) = block.next {
                    unsafe {
                        next_ptr.as_mut().prev = Some(prev_ptr);
                    }
                }
            }
        }
    }

    /// Expand the virtual and physical heap arena.
    fn grow(&mut self, grow_bytes: usize) -> bool {
        if self.current_size + grow_bytes > HEAP_MAX_SIZE {
            return false;
        }

        let grow_frames = (grow_bytes + 4095) / 4096;
        let start_virt = HEAP_START + (self.current_size as u64);

        for i in 0..grow_frames {
            let page_virt = VirtAddr::new(start_virt + ((i * 4096) as u64));
            let frame = match pmm::allocate_zeroed_frame() {
                Some(f) => f,
                None => return false,
            };

            let map_result = vmm::map_page(
                page_virt,
                frame,
                PageFlags::PRESENT | PageFlags::WRITABLE | PageFlags::NO_EXECUTE,
            );

            if map_result.is_err() {
                pmm::deallocate_frame(frame);
                return false;
            }
        }

        // Create new free block header in the newly mapped space
        let new_header_ptr = start_virt as *mut BlockHeader;
        let actual_grow = grow_frames * 4096;

        unsafe {
            new_header_ptr.write(BlockHeader {
                magic: BLOCK_MAGIC,
                is_free: true,
                size: actual_grow - HEADER_SIZE,
                padding: 0,
                next: None,
                prev: None,
            });
        }

        // Link to the end of the existing block list
        let mut last = self.head;
        while let Some(block) = last {
            let b = unsafe { block.as_ref() };
            if b.next.is_none() {
                unsafe {
                    let mut b_mut = block;
                    b_mut.as_mut().next = NonNull::new(new_header_ptr);
                    (*new_header_ptr).prev = Some(block);
                }
                break;
            }
            last = b.next;
        }

        self.current_size += actual_grow;

        // Try coalescing the new block with the previous block
        if let Some(mut prev_ptr) = unsafe { (*new_header_ptr).prev } {
            let prev = unsafe { prev_ptr.as_mut() };
            if prev.is_free {
                let prev_end = (prev as *const BlockHeader as usize) + prev.total_size();
                if prev_end == new_header_ptr as usize {
                    prev.size += actual_grow;
                    prev.next = None;
                }
            }
        }

        true
    }

    /// Retrieve total allocated bytes.
    pub fn allocated_bytes(&self) -> usize {
        self.allocated_bytes
    }

    /// Retrieve total current heap capacity.
    pub fn current_size(&self) -> usize {
        self.current_size
    }
}

// ── Global Allocator Wrapper ─────────────────────────────────────────

/// A thread-safe global allocator wrapping `HeapAllocator` with a `SpinLock`.
pub struct LockedHeap {
    inner: SpinLock<Option<HeapAllocator>>,
}

impl LockedHeap {
    /// Create an empty uninitialized allocator handle.
    pub const fn empty() -> Self {
        Self {
            inner: SpinLock::new(None),
        }
    }
}

unsafe impl GlobalAlloc for LockedHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut guard = self.inner.lock();
        if let Some(heap) = guard.as_mut() {
            heap.alloc(layout)
        } else {
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut guard = self.inner.lock();
        if let Some(heap) = guard.as_mut() {
            heap.dealloc(ptr, layout);
        }
    }
}

/// Global system allocator instance for VikingOS.
#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

// ── Subsystem Initialization ─────────────────────────────────────────

/// Initialize the kernel heap:
/// 1. Maps virtual addresses `HEAP_START .. HEAP_START + HEAP_INITIAL_SIZE`
/// 2. Initializes the Free-List allocator
/// 3. Registers the allocator globally for `alloc` crate
pub fn init() {
    let pages = HEAP_INITIAL_SIZE / 4096;

    for i in 0..pages {
        let virt = VirtAddr::new(HEAP_START + ((i * 4096) as u64));
        let frame = pmm::allocate_zeroed_frame().expect("OOM: Failed to allocate frame for kernel heap");
        vmm::map_page(
            virt,
            frame,
            PageFlags::PRESENT | PageFlags::WRITABLE | PageFlags::NO_EXECUTE,
        )
        .expect("VMM: Failed to map kernel heap page");
    }

    unsafe {
        let heap = HeapAllocator::new(VirtAddr::new(HEAP_START), HEAP_INITIAL_SIZE);
        *ALLOCATOR.inner.lock() = Some(heap);
    }

    kprintln!(
        "[  OK  ] Kernel heap initialized: {} KiB (Range: {:#018x} .. {:#018x})",
        HEAP_INITIAL_SIZE / 1024,
        HEAP_START,
        HEAP_START + (HEAP_INITIAL_SIZE as u64),
    );
}

/// Retrieve current heap statistics (allocated bytes, total capacity).
pub fn stats() -> (usize, usize) {
    let guard = ALLOCATOR.inner.lock();
    if let Some(heap) = guard.as_ref() {
        (heap.allocated_bytes(), heap.current_size())
    } else {
        (0, 0)
    }
}
