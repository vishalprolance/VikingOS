// VikingOS Kernel — I/O APIC Driver
//
// The I/O APIC manages external hardware interrupts and routes them
// to one or more Local APICs (CPUs).

#![allow(dead_code)]

/// Default I/O APIC base address.
/// In a full ACPI implementation, this would be discovered via the MADT table.
pub const IOAPIC_BASE: usize = 0xFEC00000;

// --- I/O APIC Register Offsets ---
/// IOREGSEL — Index register (write the register number to select)
const REG_SELECT: usize = 0x00;
/// IOWIN — Data window register (read/write the selected register)
const REG_WINDOW: usize = 0x10;

/// Redirection Table entries start at register index 0x10.
/// Each entry is 64 bits (two 32-bit registers).
const REDIR_TABLE_START: usize = 0x10;

pub struct IoApic {
    base: usize,
}

impl IoApic {
    /// Create a new I/O APIC handle.
    pub const fn new() -> Self {
        Self {
            base: IOAPIC_BASE,
        }
    }

    /// Retrieve the virtual base address via Higher-Half Direct Map (HHDM).
    #[inline]
    fn base_addr(&self) -> usize {
        let hhdm = crate::memory::hhdm_offset() as usize;
        if hhdm != 0 {
            hhdm + self.base
        } else {
            self.base
        }
    }

    /// Initialize the I/O APIC by ensuring its MMIO page is mapped.
    pub fn init(&self) {
        let phys = crate::memory::PhysAddr::new(self.base as u64);
        let virt = crate::memory::VirtAddr::new(self.base_addr() as u64);
        let _ = crate::memory::vmm::map_page(
            virt,
            phys,
            crate::memory::PageFlags::PRESENT
                | crate::memory::PageFlags::WRITABLE
                | crate::memory::PageFlags::NO_EXECUTE
                | crate::memory::PageFlags::CACHE_DISABLE,
        );
    }

    /// Read a 32-bit value from the I/O APIC.
    /// The I/O APIC uses an index register and a data window.
    pub unsafe fn read(&self, index: u32) -> u32 {
        unsafe {
            let base = self.base_addr();
            core::ptr::write_volatile((base + REG_SELECT) as *mut u32, index);
            core::ptr::read_volatile((base + REG_WINDOW) as *const u32)
        }
    }

    /// Write a 32-bit value to the I/O APIC.
    pub unsafe fn write(&self, index: u32, value: u32) {
        unsafe {
            let base = self.base_addr();
            core::ptr::write_volatile((base + REG_SELECT) as *mut u32, index);
            core::ptr::write_volatile((base + REG_WINDOW) as *mut u32, value);
        }
    }

    /// Maps a hardware IRQ to a LAPIC vector.
    ///
    /// # Arguments
    /// * `irq` - The hardware interrupt line (e.g., 1 for keyboard).
    /// * `vector` - The LAPIC vector to trigger (e.g., 33).
    /// * `dest_cpu` - The CPU to target (usually 0 for single-core).
    pub fn map_irq(&self, irq: u8, vector: u8, dest_cpu: u8) {
        unsafe {
            // Redirection table entry is two 32-bit registers.
            let low_reg = REDIR_TABLE_START + (irq as usize * 2);
            let high_reg = low_reg + 1;

            // Low 32 bits:
            // Bits 0-7: Vector
            // Bit 8-10: Delivery mode (000 = Fixed)
            // Bit 16: Mask (0 = enabled, 1 = masked)
            let low_val = vector as u32;
            self.write(low_reg as u32, low_val);

            // High 32 bits:
            // Bits 24-31: Destination CPU (APIC ID)
            let high_val = (dest_cpu as u32) << 24;
            self.write(high_reg as u32, high_val);
        }
    }
}

/// Global I/O APIC instance.
pub static IOAPIC: IoApic = IoApic::new();
