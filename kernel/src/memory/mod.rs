// VikingOS Kernel — Memory Management Subsystem
//
// Root module for physical memory management (PMM), virtual memory management (VMM),
// and dynamic kernel heap allocation. Provides strongly-typed physical and virtual
// addresses and Higher-Half Direct Map (HHDM) address translation.

#![allow(dead_code)]

pub mod pmm;
pub mod vmm;
pub mod heap;

use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};
use core::sync::atomic::{AtomicU64, Ordering};

pub use pmm::PhysFrame;
#[allow(unused_imports)]
pub use pmm::BitmapFrameAllocator;
#[allow(unused_imports)]
pub use vmm::{PageFlags, PageTable};
#[allow(unused_imports)]
pub use heap::LockedHeap;

/// Global Higher-Half Direct Map (HHDM) base offset provided by Limine.
static HHDM_OFFSET: AtomicU64 = AtomicU64::new(0);

/// Set the Higher-Half Direct Map (HHDM) offset.
pub fn set_hhdm_offset(offset: u64) {
    HHDM_OFFSET.store(offset, Ordering::SeqCst);
}

/// Retrieve the Higher-Half Direct Map (HHDM) offset.
#[inline(always)]
pub fn hhdm_offset() -> u64 {
    HHDM_OFFSET.load(Ordering::Relaxed)
}

/// Convert a physical address to a virtual address via HHDM.
#[inline(always)]
pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys.as_u64() + hhdm_offset())
}

/// Convert a virtual address in the HHDM region back to a physical address.
#[inline(always)]
pub fn virt_to_phys(virt: VirtAddr) -> PhysAddr {
    let offset = hhdm_offset();
    assert!(
        virt.as_u64() >= offset,
        "Cannot convert non-HHDM virtual address to physical address"
    );
    PhysAddr::new(virt.as_u64() - offset)
}

// ── Physical Address ─────────────────────────────────────────────────

/// A strongly-typed x86-64 physical memory address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct PhysAddr(pub u64);

impl PhysAddr {
    /// Create a new physical address.
    #[inline(always)]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Return the raw `u64` physical address value.
    #[inline(always)]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Return a raw const pointer to this physical address via HHDM.
    #[inline(always)]
    pub fn as_ptr<T>(self) -> *const T {
        phys_to_virt(self).as_ptr()
    }

    /// Return a raw mutable pointer to this physical address via HHDM.
    #[inline(always)]
    pub fn as_mut_ptr<T>(self) -> *mut T {
        phys_to_virt(self).as_mut_ptr()
    }

    /// Check if the physical address is aligned to a given byte boundary.
    #[inline(always)]
    pub const fn is_aligned(self, align: u64) -> bool {
        (self.0 & (align - 1)) == 0
    }

    /// Align the physical address down to the nearest multiple of `align`.
    #[inline(always)]
    pub const fn align_down(self, align: u64) -> Self {
        Self(self.0 & !(align - 1))
    }

    /// Align the physical address up to the nearest multiple of `align`.
    #[inline(always)]
    pub const fn align_up(self, align: u64) -> Self {
        Self((self.0 + align - 1) & !(align - 1))
    }

    /// Convert to a virtual address via HHDM.
    #[inline(always)]
    pub fn to_virt(self) -> VirtAddr {
        phys_to_virt(self)
    }
}

impl fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhysAddr({:#018x})", self.0)
    }
}

impl fmt::Display for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.0)
    }
}

impl fmt::LowerHex for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl Add<u64> for PhysAddr {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u64> for PhysAddr {
    #[inline(always)]
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl Sub<u64> for PhysAddr {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl SubAssign<u64> for PhysAddr {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

// ── Virtual Address ──────────────────────────────────────────────────

/// A strongly-typed x86-64 virtual memory address.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct VirtAddr(pub u64);

impl VirtAddr {
    /// Create a new virtual address.
    #[inline(always)]
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    /// Return the raw `u64` virtual address value.
    #[inline(always)]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    /// Return a raw const pointer to this virtual address.
    #[inline(always)]
    pub fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    /// Return a raw mutable pointer to this virtual address.
    #[inline(always)]
    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }

    /// Check if the virtual address is aligned to a given byte boundary.
    #[inline(always)]
    pub const fn is_aligned(self, align: u64) -> bool {
        (self.0 & (align - 1)) == 0
    }

    /// Align the virtual address down to the nearest multiple of `align`.
    #[inline(always)]
    pub const fn align_down(self, align: u64) -> Self {
        Self(self.0 & !(align - 1))
    }

    /// Align the virtual address up to the nearest multiple of `align`.
    #[inline(always)]
    pub const fn align_up(self, align: u64) -> Self {
        Self((self.0 + align - 1) & !(align - 1))
    }

    /// Convert from an HHDM virtual address to physical address.
    #[inline(always)]
    pub fn to_phys(self) -> PhysAddr {
        virt_to_phys(self)
    }

    /// PML4 index (bits 47:39).
    #[inline(always)]
    pub const fn pml4_index(self) -> usize {
        ((self.0 >> 39) & 0x1FF) as usize
    }

    /// PDPT index (bits 38:30).
    #[inline(always)]
    pub const fn pdpt_index(self) -> usize {
        ((self.0 >> 30) & 0x1FF) as usize
    }

    /// Page Directory index (bits 29:21).
    #[inline(always)]
    pub const fn pd_index(self) -> usize {
        ((self.0 >> 21) & 0x1FF) as usize
    }

    /// Page Table index (bits 20:12).
    #[inline(always)]
    pub const fn pt_index(self) -> usize {
        ((self.0 >> 12) & 0x1FF) as usize
    }

    /// Offset within 4 KiB page (bits 11:0).
    #[inline(always)]
    pub const fn page_offset(self) -> usize {
        (self.0 & 0xFFF) as usize
    }
}

impl fmt::Debug for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VirtAddr({:#018x})", self.0)
    }
}

impl fmt::Display for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#018x}", self.0)
    }
}

impl fmt::LowerHex for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl Add<u64> for VirtAddr {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u64> for VirtAddr {
    #[inline(always)]
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl Sub<u64> for VirtAddr {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl SubAssign<u64> for VirtAddr {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

// ── Subsystem Initialization ─────────────────────────────────────────

/// Initialize the entire memory management subsystem:
/// 1. Stores HHDM offset
/// 2. Initializes Physical Memory Manager (PMM)
/// 3. Initializes Virtual Memory Manager (VMM)
/// 4. Initializes Kernel Heap Allocator
pub fn init(hhdm_offset_val: u64, memmap_entries: &[&limine::memmap::Entry]) {
    set_hhdm_offset(hhdm_offset_val);

    // 1. Initialize PMM
    pmm::init(memmap_entries);

    // 2. Initialize VMM
    vmm::init();

    // 3. Initialize Kernel Heap
    heap::init();
}
