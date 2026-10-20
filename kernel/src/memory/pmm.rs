// VikingOS Kernel — Physical Memory Manager (PMM)
//
// Manages physical memory allocation using a fast bitmap frame allocator.
// Operates on 4 KiB physical page frames and ingests usable memory regions
// provided by the Limine bootloader memory map.

#![allow(dead_code)]

use crate::kprintln;
use crate::memory::{phys_to_virt, PhysAddr};
use crate::sync::SpinLock;

/// Standard x86-64 page frame size (4 KiB).
pub const PAGE_SIZE: u64 = 4096;

/// Global physical memory manager instance.
static PMM: SpinLock<Option<BitmapFrameAllocator>> = SpinLock::new(None);

/// Statistics describing current physical memory allocation.
#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    pub total_frames: usize,
    pub free_frames: usize,
    pub used_frames: usize,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
}

/// A wrapper around a physical address to represent a page frame.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(transparent)]
pub struct PhysFrame(pub PhysAddr);

impl PhysFrame {
    /// Create a new physical frame from a physical address.
    pub const fn new(addr: PhysAddr) -> Self {
        Self(addr)
    }

    /// Return the underlying physical address.
    pub const fn addr(self) -> PhysAddr {
        self.0
    }
}

impl From<PhysAddr> for PhysFrame {
    fn from(addr: PhysAddr) -> Self {
        Self(addr)
    }
}

impl From<PhysFrame> for PhysAddr {
    fn from(frame: PhysFrame) -> Self {
        frame.0
    }
}

/// A bitmap-based physical frame allocator.
pub struct BitmapFrameAllocator {
    bitmap: &'static mut [u8],
    total_frames: usize,
    free_frames: usize,
    used_frames: usize,
    last_search_byte: usize,
}

impl BitmapFrameAllocator {
    /// Mark a frame as used (set bit to 1).
    pub fn set_frame_used(&mut self, frame: usize) {
        if frame >= self.total_frames {
            return;
        }
        let byte_idx = frame / 8;
        let bit_idx = frame % 8;
        if (self.bitmap[byte_idx] & (1 << bit_idx)) == 0 {
            self.bitmap[byte_idx] |= 1 << bit_idx;
            self.free_frames = self.free_frames.saturating_sub(1);
            self.used_frames += 1;
        }
    }

    /// Mark a frame as free (clear bit to 0).
    pub fn set_frame_free(&mut self, frame: usize) {
        if frame >= self.total_frames {
            return;
        }
        let byte_idx = frame / 8;
        let bit_idx = frame % 8;
        if (self.bitmap[byte_idx] & (1 << bit_idx)) != 0 {
            self.bitmap[byte_idx] &= !(1 << bit_idx);
            self.free_frames += 1;
            self.used_frames = self.used_frames.saturating_sub(1);
            if byte_idx < self.last_search_byte {
                self.last_search_byte = byte_idx;
            }
        }
    }

    /// Check if a frame is used.
    #[inline(always)]
    pub fn is_frame_used(&self, frame: usize) -> bool {
        if frame >= self.total_frames {
            return true;
        }
        let byte_idx = frame / 8;
        let bit_idx = frame % 8;
        (self.bitmap[byte_idx] & (1 << bit_idx)) != 0
    }

    /// Allocate a single 4 KiB physical frame.
    pub fn allocate_frame(&mut self) -> Option<PhysAddr> {
        let total_bytes = self.bitmap.len();

        // Search starting from last search byte hint
        for i in 0..total_bytes {
            let byte_idx = (self.last_search_byte + i) % total_bytes;
            if self.bitmap[byte_idx] != 0xFF {
                let free_bit = (!self.bitmap[byte_idx]).trailing_zeros() as usize;
                let frame = byte_idx * 8 + free_bit;
                if frame < self.total_frames {
                    self.set_frame_used(frame);
                    self.last_search_byte = byte_idx;
                    return Some(PhysAddr::new((frame as u64) * PAGE_SIZE));
                }
            }
        }

        None
    }

    /// Allocate `count` contiguous 4 KiB physical frames.
    pub fn allocate_frames(&mut self, count: usize) -> Option<PhysAddr> {
        if count == 0 {
            return None;
        }
        if count == 1 {
            return self.allocate_frame();
        }

        let mut consecutive = 0;
        let mut start_frame = 0;

        for frame in 0..self.total_frames {
            if !self.is_frame_used(frame) {
                if consecutive == 0 {
                    start_frame = frame;
                }
                consecutive += 1;
                if consecutive == count {
                    for f in start_frame..start_frame + count {
                        self.set_frame_used(f);
                    }
                    return Some(PhysAddr::new((start_frame as u64) * PAGE_SIZE));
                }
            } else {
                consecutive = 0;
            }
        }

        None
    }

    /// Deallocate a previously allocated physical frame.
    pub fn deallocate_frame(&mut self, addr: PhysAddr) {
        let frame = (addr.as_u64() / PAGE_SIZE) as usize;
        self.set_frame_free(frame);
    }

    /// Deallocate `count` contiguous physical frames.
    pub fn deallocate_frames(&mut self, addr: PhysAddr, count: usize) {
        let start_frame = (addr.as_u64() / PAGE_SIZE) as usize;
        for f in start_frame..start_frame + count {
            self.set_frame_free(f);
        }
    }

    /// Allocate a 4 KiB frame and initialize all bytes to zero via HHDM.
    pub fn allocate_zeroed_frame(&mut self) -> Option<PhysAddr> {
        let frame = self.allocate_frame()?;
        let virt = phys_to_virt(frame);
        unsafe {
            core::ptr::write_bytes(virt.as_mut_ptr::<u8>(), 0, PAGE_SIZE as usize);
        }
        Some(frame)
    }

    /// Return memory statistics.
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            total_frames: self.total_frames,
            free_frames: self.free_frames,
            used_frames: self.used_frames,
            total_bytes: (self.total_frames as u64) * PAGE_SIZE,
            free_bytes: (self.free_frames as u64) * PAGE_SIZE,
            used_bytes: (self.used_frames as u64) * PAGE_SIZE,
        }
    }
}

// ── Global PMM API ───────────────────────────────────────────────────

/// Initialize the Physical Memory Manager using the Limine memory map.
pub fn init(memmap_entries: &[&limine::memmap::Entry]) {
    // 1. Determine highest physical address to size the frame bitmap
    let mut highest_addr: u64 = 0;
    for entry in memmap_entries.iter() {
        let top = entry.base + entry.length;
        if top > highest_addr {
            highest_addr = top;
        }
    }

    let total_frames = ((highest_addr + PAGE_SIZE - 1) / PAGE_SIZE) as usize;
    let bitmap_size_bytes = (total_frames + 7) / 8;
    let bitmap_frames_needed = ((bitmap_size_bytes as u64) + PAGE_SIZE - 1) / PAGE_SIZE;

    // 2. Find a usable region with sufficient space to host the bitmap
    let mut bitmap_phys_addr: Option<u64> = None;
    for entry in memmap_entries.iter() {
        if entry.type_ == limine::memmap::MEMMAP_USABLE
            && entry.length >= (bitmap_frames_needed * PAGE_SIZE)
        {
            bitmap_phys_addr = Some(entry.base);
            break;
        }
    }

    let bitmap_phys = bitmap_phys_addr.expect("PMM: Failed to find usable RAM for frame bitmap");
    let bitmap_virt = phys_to_virt(PhysAddr::new(bitmap_phys));
    let bitmap_slice = unsafe {
        core::slice::from_raw_parts_mut(bitmap_virt.as_mut_ptr::<u8>(), bitmap_size_bytes)
    };

    // 3. Mark all frames as used (0xFF) initially
    bitmap_slice.fill(0xFF);

    let mut allocator = BitmapFrameAllocator {
        bitmap: bitmap_slice,
        total_frames,
        free_frames: 0,
        used_frames: total_frames,
        last_search_byte: 0,
    };

    // 4. Ingest usable regions
    for entry in memmap_entries.iter() {
        if entry.type_ == limine::memmap::MEMMAP_USABLE {
            let start_frame = (entry.base / PAGE_SIZE) as usize;
            let frame_count = (entry.length / PAGE_SIZE) as usize;
            for f in start_frame..start_frame + frame_count {
                allocator.set_frame_free(f);
            }
        }
    }

    // 5. Reserve frames occupied by the bitmap itself
    let bitmap_start_frame = (bitmap_phys / PAGE_SIZE) as usize;
    for f in bitmap_start_frame..bitmap_start_frame + (bitmap_frames_needed as usize) {
        allocator.set_frame_used(f);
    }

    // 6. Reserve the first 1 MiB (frames 0..256) for BIOS/VGA/real-mode legacy hardware
    let legacy_frames = (1024 * 1024 / PAGE_SIZE) as usize;
    for f in 0..legacy_frames.min(total_frames) {
        allocator.set_frame_used(f);
    }

    let stats = allocator.stats();
    kprintln!(
        "[  OK  ] PMM initialized: {} MB total ({} frames), {} MB usable ({} frames)",
        stats.total_bytes / (1024 * 1024),
        stats.total_frames,
        stats.free_bytes / (1024 * 1024),
        stats.free_frames,
    );

    *PMM.lock() = Some(allocator);
}

/// Allocate a 4 KiB physical frame from the global PMM.
pub fn allocate_frame() -> Option<PhysAddr> {
    PMM.lock().as_mut()?.allocate_frame()
}

/// Allocate `count` contiguous 4 KiB physical frames from the global PMM.
pub fn allocate_frames(count: usize) -> Option<PhysAddr> {
    PMM.lock().as_mut()?.allocate_frames(count)
}

/// Deallocate a physical frame back to the global PMM.
pub fn deallocate_frame(addr: PhysAddr) {
    if let Some(pmm) = PMM.lock().as_mut() {
        pmm.deallocate_frame(addr);
    }
}

/// Deallocate `count` contiguous physical frames.
pub fn deallocate_frames(addr: PhysAddr, count: usize) {
    if let Some(pmm) = PMM.lock().as_mut() {
        pmm.deallocate_frames(addr, count);
    }
}

/// Allocate a 4 KiB frame and zero its content via HHDM.
pub fn allocate_zeroed_frame() -> Option<PhysAddr> {
    PMM.lock().as_mut()?.allocate_zeroed_frame()
}

/// Retrieve current physical memory statistics.
pub fn get_stats() -> MemoryStats {
    PMM.lock()
        .as_ref()
        .map(|pmm| pmm.stats())
        .unwrap_or(MemoryStats {
            total_frames: 0,
            free_frames: 0,
            used_frames: 0,
            total_bytes: 0,
            free_bytes: 0,
            used_bytes: 0,
        })
}
