// VikingOS Kernel — Interrupt Descriptor Table (IDT)
//
// The IDT defines how the x86-64 CPU responds to hardware interrupts,
// processor exceptions, and software interrupts. It contains 256 gate
// descriptors that map vector numbers (0..255) to handler routines.

use core::mem::size_of;

/// Total number of entries in the x86-64 IDT
pub const IDT_ENTRIES: usize = 256;

/// Gate type flag for 64-bit Interrupt Gate (Ring 0, interrupts disabled on entry)
pub const IDT_FLAG_INTERRUPT_GATE: u8 = 0x8E;

/// Gate type flag for 64-bit User Trap Gate (Ring 3 callable, e.g., Breakpoint int3)
pub const IDT_FLAG_USER_TRAP_GATE: u8 = 0xEE;

/// 64-bit IDT Gate Descriptor (16 bytes)
#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct IdtEntry {
    pub offset_low: u16,        // bits 0..15 of target address
    pub selector: u16,          // code segment selector (0x08)
    pub ist: u8,                // bits 0..2: IST stack index (0=none, 1..7=IST1..7)
    pub flags: u8,              // gate type, DPL, and present flag
    pub offset_mid: u16,        // bits 16..31 of target address
    pub offset_high: u32,       // bits 32..63 of target address
    pub reserved: u32,          // must be zero
}

impl IdtEntry {
    /// Create an empty/unpresent IDT entry
    pub const NULL: Self = Self {
        offset_low: 0,
        selector: 0,
        ist: 0,
        flags: 0,
        offset_mid: 0,
        offset_high: 0,
        reserved: 0,
    };

    /// Create an active IDT gate descriptor pointing to a 64-bit handler address
    pub fn new(handler: usize, selector: u16, ist: u8, flags: u8) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector,
            ist: ist & 0x07,
            flags,
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }
}

/// The 256-entry Interrupt Descriptor Table
#[repr(C, align(16))]
pub struct InterruptDescriptorTable {
    pub entries: [IdtEntry; IDT_ENTRIES],
}

impl InterruptDescriptorTable {
    /// Create an uninitialized IDT
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::NULL; IDT_ENTRIES],
        }
    }

    /// Register an interrupt or exception handler for a specific vector
    pub fn set_handler(&mut self, index: usize, handler: usize, ist: u8, flags: u8) {
        if index < IDT_ENTRIES {
            self.entries[index] = IdtEntry::new(
                handler,
                super::gdt::KERNEL_CODE_SELECTOR,
                ist,
                flags,
            );
        }
    }

    /// Load this IDT into the CPU using the `lidt` assembly instruction
    pub fn load(&'static self) {
        #[repr(C, packed)]
        struct IdtPointer {
            limit: u16,
            base: u64,
        }

        let ptr = IdtPointer {
            limit: (size_of::<[IdtEntry; IDT_ENTRIES]>() - 1) as u16,
            base: self.entries.as_ptr() as u64,
        };

        unsafe {
            core::arch::asm!(
                "lidt [{0}]",
                in(reg) &ptr,
                options(readonly, nostack, preserves_flags),
            );
        }
    }
}

/// Kernel singleton IDT
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

/// Return a mutable reference to the kernel IDT
pub fn get_idt() -> &'static mut InterruptDescriptorTable {
    unsafe { &mut *core::ptr::addr_of_mut!(IDT) }
}
