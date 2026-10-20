// VikingOS Kernel — Virtual Memory Manager (VMM) & Paging
//
// Implements 4-level x86-64 paging (PML4, PDPT, PD, PT), page mapping,
// unmapping, virtual-to-physical address translation, and TLB invalidation.

#![allow(dead_code)]

use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use crate::kprintln;
use crate::memory::{phys_to_virt, pmm, PhysAddr, VirtAddr};
use crate::sync::SpinLock;

/// Global spinlock synchronizing page table modifications.
static VMM_LOCK: SpinLock<()> = SpinLock::new(());

// ── Page Flags ───────────────────────────────────────────────────────

/// Page table entry permission and status flags.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct PageFlags(pub u64);

impl PageFlags {
    pub const NONE: Self = Self(0);
    pub const PRESENT: Self = Self(1 << 0);
    pub const WRITABLE: Self = Self(1 << 1);
    pub const USER_ACCESSIBLE: Self = Self(1 << 2);
    pub const WRITE_THROUGH: Self = Self(1 << 3);
    pub const CACHE_DISABLE: Self = Self(1 << 4);
    pub const ACCESSED: Self = Self(1 << 5);
    pub const DIRTY: Self = Self(1 << 6);
    pub const HUGE_PAGE: Self = Self(1 << 7);
    pub const GLOBAL: Self = Self(1 << 8);
    pub const NO_EXECUTE: Self = Self(1 << 63);

    /// Check if all specified flags are present.
    #[inline(always)]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl BitOr for PageFlags {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for PageFlags {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for PageFlags {
    type Output = Self;
    #[inline(always)]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for PageFlags {
    #[inline(always)]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for PageFlags {
    type Output = Self;
    #[inline(always)]
    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

// ── Page Table Entry ─────────────────────────────────────────────────

/// An 8-byte 64-bit page table entry in an x86-64 page table.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct PageTableEntry(pub u64);

const ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

impl PageTableEntry {
    /// Return the physical address mapped by this entry.
    #[inline(always)]
    pub fn phys_addr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & ADDR_MASK)
    }

    /// Return the page flags encoded in this entry.
    #[inline(always)]
    pub fn flags(&self) -> PageFlags {
        PageFlags(self.0 & !ADDR_MASK)
    }

    /// Check if the page is present in memory.
    #[inline(always)]
    pub fn is_present(&self) -> bool {
        (self.0 & PageFlags::PRESENT.0) != 0
    }

    /// Check if this entry maps a huge page (2 MiB or 1 GiB).
    #[inline(always)]
    pub fn is_huge(&self) -> bool {
        (self.0 & PageFlags::HUGE_PAGE.0) != 0
    }

    /// Set entry to target physical address and flags.
    #[inline(always)]
    pub fn set(&mut self, phys: PhysAddr, flags: PageFlags) {
        self.0 = (phys.as_u64() & ADDR_MASK) | flags.0 | PageFlags::PRESENT.0;
    }

    /// Clear this entry.
    #[inline(always)]
    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

// ── Page Table Structure ─────────────────────────────────────────────

/// A 4096-byte 512-entry x86-64 page table (PML4, PDPT, PD, or PT).
#[derive(Clone, Copy)]
#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; 512],
}

impl PageTable {
    /// Create a zero-initialized page table.
    pub const fn empty() -> Self {
        Self {
            entries: [PageTableEntry(0); 512],
        }
    }

    /// Clear all entries in the table.
    pub fn zero(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.clear();
        }
    }
}

// ── CPU Register & Instruction Helpers ───────────────────────────────

/// Read physical address of active Level 4 Page Table from CR3.
#[inline(always)]
pub fn read_cr3() -> PhysAddr {
    let cr3: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr3",
            out(reg) cr3,
            options(nomem, nostack, preserves_flags)
        );
    }
    PhysAddr::new(cr3 & ADDR_MASK)
}

/// Invalidate a virtual address in the Translation Lookaside Buffer (TLB).
#[inline(always)]
pub fn invlpg(virt: VirtAddr) {
    unsafe {
        core::arch::asm!(
            "invlpg [{}]",
            in(reg) virt.as_u64(),
            options(nostack, preserves_flags)
        );
    }
}

// ── Errors ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    OutOfMemory,
    AlreadyMapped,
    HugePageEncountered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmapError {
    NotMapped,
    HugePageEncountered,
}

// ── Page Table Operations ────────────────────────────────────────────

/// Retrieve mutable reference to active Level 4 Page Table (PML4) via HHDM.
fn active_pml4() -> &'static mut PageTable {
    let cr3 = read_cr3();
    let virt = phys_to_virt(cr3);
    unsafe { &mut *virt.as_mut_ptr::<PageTable>() }
}

/// Map a 4 KiB virtual page to a physical frame in the active address space.
pub fn map_page(virt: VirtAddr, phys: PhysAddr, flags: PageFlags) -> Result<(), MapError> {
    let _guard = VMM_LOCK.lock();
    let pml4 = active_pml4();

    // Level 4: PML4
    let pml4_idx = virt.pml4_index();
    let pml4_entry = &mut pml4.entries[pml4_idx];
    let pdpt: &mut PageTable = if !pml4_entry.is_present() {
        let frame = pmm::allocate_zeroed_frame().ok_or(MapError::OutOfMemory)?;
        pml4_entry.set(
            frame,
            PageFlags::PRESENT | PageFlags::WRITABLE | (flags & PageFlags::USER_ACCESSIBLE),
        );
        unsafe { &mut *phys_to_virt(frame).as_mut_ptr::<PageTable>() }
    } else {
        unsafe { &mut *phys_to_virt(pml4_entry.phys_addr()).as_mut_ptr::<PageTable>() }
    };

    // Level 3: PDPT
    let pdpt_idx = virt.pdpt_index();
    let pdpt_entry = &mut pdpt.entries[pdpt_idx];
    if pdpt_entry.is_huge() {
        return Err(MapError::HugePageEncountered);
    }
    let pd: &mut PageTable = if !pdpt_entry.is_present() {
        let frame = pmm::allocate_zeroed_frame().ok_or(MapError::OutOfMemory)?;
        pdpt_entry.set(
            frame,
            PageFlags::PRESENT | PageFlags::WRITABLE | (flags & PageFlags::USER_ACCESSIBLE),
        );
        unsafe { &mut *phys_to_virt(frame).as_mut_ptr::<PageTable>() }
    } else {
        unsafe { &mut *phys_to_virt(pdpt_entry.phys_addr()).as_mut_ptr::<PageTable>() }
    };

    // Level 2: Page Directory (PD)
    let pd_idx = virt.pd_index();
    let pd_entry = &mut pd.entries[pd_idx];
    if pd_entry.is_huge() {
        return Err(MapError::HugePageEncountered);
    }
    let pt: &mut PageTable = if !pd_entry.is_present() {
        let frame = pmm::allocate_zeroed_frame().ok_or(MapError::OutOfMemory)?;
        pd_entry.set(
            frame,
            PageFlags::PRESENT | PageFlags::WRITABLE | (flags & PageFlags::USER_ACCESSIBLE),
        );
        unsafe { &mut *phys_to_virt(frame).as_mut_ptr::<PageTable>() }
    } else {
        unsafe { &mut *phys_to_virt(pd_entry.phys_addr()).as_mut_ptr::<PageTable>() }
    };

    // Level 1: Page Table (PT)
    let pt_idx = virt.pt_index();
    let pt_entry = &mut pt.entries[pt_idx];
    if pt_entry.is_present() {
        return Err(MapError::AlreadyMapped);
    }

    pt_entry.set(phys, flags | PageFlags::PRESENT);
    invlpg(virt);

    Ok(())
}

/// Unmap a 4 KiB virtual page, invalidating the TLB and returning the physical address.
pub fn unmap_page(virt: VirtAddr) -> Result<PhysAddr, UnmapError> {
    let _guard = VMM_LOCK.lock();
    let pml4 = active_pml4();

    let pml4_entry = &pml4.entries[virt.pml4_index()];
    if !pml4_entry.is_present() {
        return Err(UnmapError::NotMapped);
    }

    let pdpt = unsafe { &mut *phys_to_virt(pml4_entry.phys_addr()).as_mut_ptr::<PageTable>() };
    let pdpt_entry = &pdpt.entries[virt.pdpt_index()];
    if !pdpt_entry.is_present() {
        return Err(UnmapError::NotMapped);
    }
    if pdpt_entry.is_huge() {
        return Err(UnmapError::HugePageEncountered);
    }

    let pd = unsafe { &mut *phys_to_virt(pdpt_entry.phys_addr()).as_mut_ptr::<PageTable>() };
    let pd_entry = &pd.entries[virt.pd_index()];
    if !pd_entry.is_present() {
        return Err(UnmapError::NotMapped);
    }
    if pd_entry.is_huge() {
        return Err(UnmapError::HugePageEncountered);
    }

    let pt = unsafe { &mut *phys_to_virt(pd_entry.phys_addr()).as_mut_ptr::<PageTable>() };
    let pt_entry = &mut pt.entries[virt.pt_index()];
    if !pt_entry.is_present() {
        return Err(UnmapError::NotMapped);
    }

    let phys = pt_entry.phys_addr();
    pt_entry.clear();
    invlpg(virt);

    Ok(phys)
}

/// Translate a virtual address to its mapped physical address.
pub fn translate(virt: VirtAddr) -> Option<PhysAddr> {
    let _guard = VMM_LOCK.lock();
    let pml4 = active_pml4();

    let pml4_entry = &pml4.entries[virt.pml4_index()];
    if !pml4_entry.is_present() {
        return None;
    }

    let pdpt = unsafe { &*phys_to_virt(pml4_entry.phys_addr()).as_ptr::<PageTable>() };
    let pdpt_entry = &pdpt.entries[virt.pdpt_index()];
    if !pdpt_entry.is_present() {
        return None;
    }
    // 1 GiB huge page check
    if pdpt_entry.is_huge() {
        let base = pdpt_entry.phys_addr().as_u64() & !0x3FFF_FFFF;
        let offset = virt.as_u64() & 0x3FFF_FFFF;
        return Some(PhysAddr::new(base + offset));
    }

    let pd = unsafe { &*phys_to_virt(pdpt_entry.phys_addr()).as_ptr::<PageTable>() };
    let pd_entry = &pd.entries[virt.pd_index()];
    if !pd_entry.is_present() {
        return None;
    }
    // 2 MiB huge page check
    if pd_entry.is_huge() {
        let base = pd_entry.phys_addr().as_u64() & !0x1F_FFFF;
        let offset = virt.as_u64() & 0x1F_FFFF;
        return Some(PhysAddr::new(base + offset));
    }

    let pt = unsafe { &*phys_to_virt(pd_entry.phys_addr()).as_ptr::<PageTable>() };
    let pt_entry = &pt.entries[virt.pt_index()];
    if !pt_entry.is_present() {
        return None;
    }

    Some(PhysAddr::new(pt_entry.phys_addr().as_u64() + (virt.page_offset() as u64)))
}

/// Initialize the Virtual Memory Manager.
pub fn init() {
    let cr3 = read_cr3();
    kprintln!("[  OK  ] VMM initialized: 4-level paging active (PML4: {:#018x})", cr3.as_u64());
}
