// VikingOS Kernel — Local APIC (LAPIC) Driver
//
// The Local APIC manages interrupts for the local CPU core.
// This driver handles initialization, EOI signaling, and timer configuration.

#![allow(dead_code)]


/// LAPIC base address (standard for x86-64).
/// In a real system, this should be verified via ACPI MADT, but 0xFEE00000 is the standard.
pub const LAPIC_BASE: usize = 0xFEE00000;

// --- LAPIC Register Offsets ---
const REG_ID: u16 = 0x020;
const REG_EOI: u16 = 0x0B0;
const REG_SVR: u16 = 0x0F0;
const REG_ICR_LOW: u16 = 0x300;
const REG_ICR_HIGH: u16 = 0x310;
const REG_LVT_TIMER: u16 = 0x320;
const REG_TIMER_INIT_COUNT: u16 = 0x380;
const REG_TIMER_CUR_COUNT: u16 = 0x390;

/// Spurious Interrupt Vector.
/// Must be set to a vector that is not used by other hardware interrupts.
const SPURIOUS_VECTOR: u8 = 0xFF;

pub struct LocalApic {
    base: usize,
}

impl LocalApic {
    /// Create a new LAPIC handle.
    pub const fn new() -> Self {
        Self {
            base: LAPIC_BASE,
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

    /// Read a 32-bit value from a LAPIC register.
    unsafe fn read_reg(&self, offset: u16) -> u32 {
        // LAPIC registers are memory-mapped. Access via HHDM virtual address.
        unsafe {
            core::ptr::read_volatile((self.base_addr() + offset as usize) as *const u32)
        }
    }

    /// Write a 32-bit value to a LAPIC register.
    unsafe fn write_reg(&self, offset: u16, value: u32) {
        unsafe {
            core::ptr::write_volatile((self.base_addr() + offset as usize) as *mut u32, value);
        }
    }

    /// Initialize the LAPIC.
    pub fn init(&self) {
        // 1. Ensure Local APIC MMIO page is mapped in VMM page tables
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

        unsafe {
            // 2. Ensure APIC Global Enable bit (bit 11) is set in IA32_APIC_BASE MSR (0x1B)
            let mut eax: u32;
            let mut edx: u32;
            core::arch::asm!("rdmsr", in("ecx") 0x1Bu32, out("eax") eax, out("edx") edx, options(nomem, nostack));
            if (eax & (1 << 11)) == 0 {
                eax |= 1 << 11;
                core::arch::asm!("wrmsr", in("ecx") 0x1Bu32, in("eax") eax, in("edx") edx, options(nomem, nostack));
            }

            // 3. Set Divide Configuration Register (0x3E0) to Divide by 16 (0x03)
            self.write_reg(0x3E0, 0x03);

            // 4. Set Spurious Interrupt Vector Register (SVR) with Bit 8 (APIC Software Enable)
            let svr_val = (SPURIOUS_VECTOR as u32) | (1 << 8);
            self.write_reg(REG_SVR, svr_val);
        }
    }

    /// Send an End of Interrupt (EOI) signal to the LAPIC.
    /// This tells the LAPIC that the current interrupt has been handled.
    pub fn send_eoi(&self) {
        unsafe {
            // EOI register is at offset 0x0B0. Writing any value signals EOI.
            self.write_reg(REG_EOI, 0);
        }
    }

    /// Configure the LAPIC Timer.
    ///
    /// # Arguments
    /// * `vector` - The IDT vector to trigger on timer expiration.
    /// * `frequency_hz` - The desired tick rate (e.g., 100 for 100Hz).
    pub fn setup_timer(&self, vector: u8, frequency_hz: u32) {
        unsafe {
            // 1. Set Divide Configuration Register (0x3E0) to Divide by 16 (0x03)
            self.write_reg(0x3E0, 0x03);

            // 2. Set the LVT Timer register:
            // - Periodic mode (bit 17 = 1)
            // - Vector in bits 0-7
            let lvt_val = (vector as u32) | (1 << 17);
            self.write_reg(REG_LVT_TIMER, lvt_val);

            // 3. Calculate initial count.
            // With divide by 16 on a typical virtual machine bus clock (~10-100 MHz):
            let count = (10_000_000 / 16) / frequency_hz.max(1);
            self.write_reg(REG_TIMER_INIT_COUNT, count.max(1000));
        }
    }
}

/// Global LAPIC instance.
pub static LAPIC: LocalApic = LocalApic::new();
